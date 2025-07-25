use std::{cmp::Ordering, collections::HashSet, env};

use crate::{
    handler::{
        Crateinfo, DependencyCount, DependencyCrateInfo, DependencyInfo, DependentCount,
        DependentData, DependentInfo, NewRustsec, RustSec, Versionpage,
    },
    UploadedCrate, Userinfo,
};
use chrono::NaiveDateTime;
use model::tugraph_model::{Program, UProgram};
use semver::Version;
use serde::{Deserialize, Serialize};
use tokio_postgres::{Error, NoTls};
use utoipa::ToSchema;
pub struct DBHandler {
    pub client: tokio_postgres::Client,
}
#[derive(Serialize, Deserialize, Debug, Clone, ToSchema)]
pub struct CveInfo {
    cve_id: String,
    url: String,
    description: String,
    crate_name: String,
    start_version: String,
    end_version: String,
}
#[derive(Serialize, Deserialize, Debug, Clone, ToSchema)]
pub struct Allcve {
    cves: Vec<CveInfo>,
}

pub fn db_connection_config_from_env() -> String {
    format!(
        "host={} port={} user={} password={} dbname={}",
        env::var("POSTGRES_HOST_IP").unwrap(),
        env::var("POSTGRES_HOST_PORT").unwrap(),
        env::var("POSTGRES_USER_NAME").unwrap(),
        env::var("POSTGRES_USER_PASSWORD").unwrap(),
        env::var("POSTGRES_CRATESPRO_DB").unwrap()
    )
}
pub fn db_cratesio_connection_config_from_env() -> String {
    format!(
        "host={} port={} user={} password={} dbname={}",
        env::var("POSTGRES_HOST_IP").unwrap(),
        env::var("POSTGRES_HOST_PORT").unwrap(),
        env::var("POSTGRES_USER_NAME").unwrap(),
        env::var("POSTGRES_USER_PASSWORD").unwrap(),
        env::var("POSTGRES_CRATESIO_DB").unwrap()
    )
}

impl DBHandler {
    pub async fn connect() -> Result<Self, Error> {
        let db_connection_config = db_connection_config_from_env();
        let (client, connection) = tokio_postgres::connect(&db_connection_config, NoTls).await?;

        // Spawn the connection on a separate task
        tokio::spawn(async move {
            if let Err(e) = connection.await {
                eprintln!("connection error: {}", e);
            }
        });

        // 创建 cratespro 数据库
        client
            .execute("CREATE DATABASE cratespro", &[])
            .await
            .or_else(|err| {
                if let Some(db_err) = err.as_db_error() {
                    if db_err.code() == &tokio_postgres::error::SqlState::DUPLICATE_DATABASE {
                        return Ok(0);
                    }
                }
                Err(err)
            })?;

        // 重新连接到 cratespro 数据库
        let (client, connection) = tokio_postgres::connect(&db_connection_config, NoTls).await?;

        // Spawn the connection on a separate task
        tokio::spawn(async move {
            if let Err(e) = connection.await {
                eprintln!("connection error: {}", e);
            }
        });

        Ok(DBHandler { client })
    }

    pub async fn clear_database(&self) -> Result<(), Error> {
        self.client
            .batch_execute(
                "
                DO $$
                BEGIN
                    IF EXISTS (SELECT 1 FROM pg_tables WHERE tablename = 'programs') THEN
                        DROP TABLE programs CASCADE;
                    END IF;


                    IF EXISTS (SELECT 1 FROM pg_tables WHERE tablename = 'program_versions') THEN
                        DROP TABLE program_versions CASCADE;
                    END IF;

                    IF EXISTS (SELECT 1 FROM pg_tables WHERE tablename = 'program_dependencies') THEN
                        DROP TABLE program_dependencies CASCADE;
                    END IF;
                    

                END $$;
                ",
            )
            .await
    }
    
    /// 从PostgreSQL数据库中查询并获取所有CVE记录的列表
    pub async fn create_tables(&self) -> Result<(), Error> {
        let create_programs_table = "
            CREATE TABLE IF NOT EXISTS programs (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                description TEXT,
                namespace TEXT,
                max_version TEXT,
                github_url TEXT,
                mega_url TEXT,
                doc_url TEXT,
                program_type TEXT NOT NULL,
                downloads BIGINT,
                cratesio TEXT
            );
        ";

        let create_program_versions_table = "
            CREATE TABLE IF NOT EXISTS program_versions (
                name_and_version TEXT PRIMARY KEY,
                id TEXT NOT NULL,
                name TEXT NOT NULL,
                version TEXT NOT NULL,
                documentation TEXT,
                version_type TEXT NOT NULL,
                created_at TIMESTAMPTZ DEFAULT NOW()
            );
        ";

        let create_program_dependencies_table = "
            CREATE TABLE IF NOT EXISTS program_dependencies (
                name_and_version TEXT NOT NULL,
                dependency_name TEXT NOT NULL,
                dependency_version TEXT NOT NULL,
                PRIMARY KEY (name_and_version, dependency_name, dependency_version)
            );
        ";

        // 执行创建表的 SQL 语句
        let result = self
            .client
            .batch_execute(&format!(
                "{}{}{}",
                create_programs_table,
                create_program_versions_table,
                create_program_dependencies_table
            ))
            .await;

        match result {
            Ok(_) => {
                tracing::info!("Tables created successfully.");
                Ok(())
            }
            Err(e) => {
                tracing::error!("Error creating tables: {:?}", e);
                Err(e)
            }
        }
    }
    
    /// 将程序数据插入到PostgreSQL数据库中
    pub async fn insert_program_data(
        &self,
        program: Program,
        uprogram: UProgram,
        _versions: Vec<crate::VersionInfo>,
    ) -> Result<(), Error> {
        let (program_type, downloads, cratesio) = match &uprogram {
            UProgram::Library(lib) => ("Library", Some(lib.downloads), lib.cratesio.clone()),
            UProgram::Application(_) => ("Application", None, None),
        };

        self.client
            .execute(
                "
            INSERT INTO programs (
                id, name, description, namespace, 
                max_version, github_url, mega_url, doc_url,
                program_type, downloads, cratesio
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
            ",
                &[
                    &program.id,
                    &program.name,
                    &program.description.unwrap_or_default(),
                    &program.namespace.unwrap_or_default(),
                    &program.max_version.unwrap_or_default(),
                    &program.github_url.unwrap_or_default(),
                    &program.mega_url.unwrap_or_default(),
                    &program.doc_url.unwrap_or_default(),
                    &program_type,
                    &downloads.unwrap_or_default(),
                    &cratesio.unwrap_or_default(),
                ],
            )
            .await
            .map_err(|e| {
                eprintln!("Error inserting program: {:?}", e);
                e
            })
            .unwrap();

        tracing::info!("finish to insert program.");

        // 插入 UVersion 数据
        /*for version in versions {
            let name_and_version = version.version_base.get_name_and_version();

            match version.version_base {
                UVersion::LibraryVersion(lib_ver) => {
                    self.client
                        .execute(
                            "
                        INSERT INTO program_versions (
                            name_and_version, id, name, version,
                            documentation, version_type, created_at
                        ) VALUES ($1, $2, $3, $4, $5, $6, NOW())
                        ",
                            &[
                                &lib_ver.name_and_version,
                                &lib_ver.id,
                                &lib_ver.name,
                                &lib_ver.version,
                                &Some(lib_ver.documentation),
                                &"LibraryVersion",
                            ],
                        )
                        .await
                        .unwrap();
                }
                UVersion::ApplicationVersion(app_ver) => {
                    self.client
                        .execute(
                            "
                        INSERT INTO program_versions (
                            name_and_version, id, name, version,
                            documentation, version_type, created_at
                        ) VALUES ($1, $2, $3, $4, $5, $6, NOW())
                        ",
                            &[
                                &app_ver.name_and_version,
                                &app_ver.id,
                                &app_ver.name,
                                &app_ver.version,
                                &None::<String>, // ApplicationVersion 没有 documentation 字段
                                &"ApplicationVersion",
                            ],
                        )
                        .await
                        .unwrap();
                }
            }

            // 插入该版本的所有依赖项
            for dep in version.dependencies {
                self.client
                    .execute(
                        "
                        INSERT INTO program_dependencies (
                            name_and_version, dependency_name, dependency_version
                        ) VALUES ($1, $2, $3)
                        ",
                        &[&name_and_version, &dep.name, &dep.version],
                    )
                    .await?;
            }
        }*/
        //tracing::info!("Finish to insert all versions.");

        Ok(())
    }

    /// 从PostgreSQL数据库中查询并获取所有CVE记录的列表
    pub async fn get_all_cvelist(&self) -> Result<Allcve, Error> {
        //let getcve = "SELECT cve_id, name, start_version, end_version FROM cves;";

        let raws = self
            .client
            .query(
                "SELECT cve_id, name, start_version, end_version,description FROM cves;",
                &[],
            )
            .await?;
        let mut getcves = vec![];
        for raw in raws {
            let front = "https://www.cve.org/CVERecord?id=";
            let cve_id: String = raw.get(0);
            let cve_url = front.to_string() + &cve_id;
            let cve_info = CveInfo {
                cve_id: raw.get(0),
                url: cve_url,
                description: raw.get(4),
                crate_name: raw.get(1),
                start_version: raw.get(2),
                end_version: raw.get(3),
            };
            getcves.push(cve_info);
        }
        let res = Allcve { cves: getcves };

        Ok(res)
    }

    /// 检查给定的版本号是否落在指定的版本范围区间内
    pub async fn process_closed_interval_of_match_version(
        &self,
        oneline_patched: String,
        version: String,
    ) -> Result<bool, Error> {
        let mut matched = false;
        let mut two_versions = vec![];
        let newparts: Vec<&str> = oneline_patched.split(',').collect();
        for part in newparts {
            let one_version = part.to_string();
            let res_one_version = one_version.trim();
            two_versions.push(res_one_version.to_string());
        }
        let mut left = "".to_string();
        let mut right = "".to_string();
        if two_versions.len() == 2 {
            if two_versions[0].clone().starts_with(">") || two_versions[0].clone().starts_with(">=")
            {
                left = two_versions[0].clone();
                right = two_versions[1].clone();
            } else if two_versions[0].clone().starts_with("<")
                || two_versions[0].clone().starts_with("<=")
            {
                left = two_versions[1].clone();
                right = two_versions[0].clone();
            }
        }
        if (left.starts_with(">") && !left.starts_with(">="))
            && (right.starts_with("<") && !right.starts_with("<="))
        {
            //> <
            let mut versions = vec![];
            let tmp_left = &left[1..];
            let left_version = tmp_left.to_string();
            let tmp_right = &right[1..];
            let right_version = tmp_right.to_string();
            versions.push(version.clone());
            versions.push(left_version.clone());
            versions.push(right_version.clone());
            versions.sort_by(|a, b| {
                let version_a = Version::parse(a);
                let version_b = Version::parse(b);
                match (version_a, version_b) {
                    (Ok(v_a), Ok(v_b)) => v_b.cmp(&v_a),
                    (Ok(_), Err(_)) => Ordering::Less,
                    (Err(_), Ok(_)) => Ordering::Greater,
                    (Err(_), Err(_)) => Ordering::Equal,
                }
            });
            if version.clone() == versions[1].clone()
                && (versions[0].clone() != version.clone()
                    && versions[2].clone() != version.clone())
            {
                matched = true;
            }
        } else if (left.starts_with(">") && !left.starts_with(">=")) && right.starts_with("<=") {
            //> <=
            let mut versions = vec![];
            let tmp_left = &left[1..];
            let left_version = tmp_left.to_string();
            let tmp_right = &right[2..];
            let right_version = tmp_right.to_string();
            versions.push(version.clone());
            versions.push(left_version.clone());
            versions.push(right_version.clone());
            versions.sort_by(|a, b| {
                let version_a = Version::parse(a);
                let version_b = Version::parse(b);
                match (version_a, version_b) {
                    (Ok(v_a), Ok(v_b)) => v_b.cmp(&v_a),
                    (Ok(_), Err(_)) => Ordering::Less,
                    (Err(_), Ok(_)) => Ordering::Greater,
                    (Err(_), Err(_)) => Ordering::Equal,
                }
            });
            if version.clone() == versions[1].clone() && (versions[2].clone() != version.clone()) {
                matched = true;
            }
        } else if left.starts_with(">=") && (right.starts_with("<") && !right.starts_with("<=")) {
            //>= <
            let mut versions = vec![];
            let tmp_left = &left[2..];
            let left_version = tmp_left.to_string();
            let tmp_right = &right[1..];
            let right_version = tmp_right.to_string();
            versions.push(version.clone());
            versions.push(left_version.clone());
            versions.push(right_version.clone());
            versions.sort_by(|a, b| {
                let version_a = Version::parse(a);
                let version_b = Version::parse(b);
                match (version_a, version_b) {
                    (Ok(v_a), Ok(v_b)) => v_b.cmp(&v_a),
                    (Ok(_), Err(_)) => Ordering::Less,
                    (Err(_), Ok(_)) => Ordering::Greater,
                    (Err(_), Err(_)) => Ordering::Equal,
                }
            });
            if version.clone() == versions[1].clone() && (versions[0].clone() != version.clone()) {
                matched = true;
            }
        } else if left.starts_with(">=") && right.starts_with("<=") {
            //>= <=
            let mut versions = vec![];
            let tmp_left = &left[2..];
            let left_version = tmp_left.to_string();
            let tmp_right = &right[2..];
            let right_version = tmp_right.to_string();
            versions.push(version.clone());
            versions.push(left_version.clone());
            versions.push(right_version.clone());
            versions.sort_by(|a, b| {
                let version_a = Version::parse(a);
                let version_b = Version::parse(b);
                match (version_a, version_b) {
                    (Ok(v_a), Ok(v_b)) => v_b.cmp(&v_a),
                    (Ok(_), Err(_)) => Ordering::Less,
                    (Err(_), Ok(_)) => Ordering::Greater,
                    (Err(_), Err(_)) => Ordering::Equal,
                }
            });
            if version.clone() == versions[1].clone() {
                matched = true;
            }
        }
        Ok(matched)
    }

    /// 检查一个版本是否满足单个约束条件
    pub async fn process_open_interval_of_match_version(
        &self,
        oneline_patched: String,
        version: String,
    ) -> Result<bool, Error> {
        let mut matched = false;
        if oneline_patched.starts_with(">") && !oneline_patched.starts_with(">=") {
            let mut versions = vec![];
            let trimmed = &oneline_patched[1..];
            let res = trimmed.to_string();
            versions.push(version.clone());
            versions.push(res.clone());
            versions.sort_by(|a, b| {
                let version_a = Version::parse(a);
                let version_b = Version::parse(b);
                match (version_a, version_b) {
                    (Ok(v_a), Ok(v_b)) => v_b.cmp(&v_a),
                    (Ok(_), Err(_)) => Ordering::Less,
                    (Err(_), Ok(_)) => Ordering::Greater,
                    (Err(_), Err(_)) => Ordering::Equal,
                }
            });
            if versions[0].clone() == version.clone() && res.clone() != version.clone() {
                matched = true;
            }
        } else if let Some(trimmed) = oneline_patched.strip_prefix(">=") {
            let mut versions = vec![];
            let res = trimmed.to_string();
            versions.push(version.clone());
            versions.push(res.clone());
            versions.sort_by(|a, b| {
                let version_a = Version::parse(a);
                let version_b = Version::parse(b);
                match (version_a, version_b) {
                    (Ok(v_a), Ok(v_b)) => v_b.cmp(&v_a),
                    (Ok(_), Err(_)) => Ordering::Less,
                    (Err(_), Ok(_)) => Ordering::Greater,
                    (Err(_), Err(_)) => Ordering::Equal,
                }
            });
            if versions[0].clone() == version.clone() {
                matched = true;
            }
        } else if oneline_patched.starts_with("<") && !oneline_patched.starts_with("<=") {
            let mut versions = vec![];
            let trimmed = &oneline_patched[1..];
            let res = trimmed.to_string();
            versions.push(version.clone());
            versions.push(res.clone());
            versions.sort_by(|a, b| {
                let version_a = Version::parse(a);
                let version_b = Version::parse(b);
                match (version_a, version_b) {
                    (Ok(v_a), Ok(v_b)) => v_b.cmp(&v_a),
                    (Ok(_), Err(_)) => Ordering::Less,
                    (Err(_), Ok(_)) => Ordering::Greater,
                    (Err(_), Err(_)) => Ordering::Equal,
                }
            });
            if versions[1].clone() == version.clone() && res.clone() != version.clone() {
                matched = true;
            }
        } else if let Some(trimmed) = oneline_patched.strip_prefix("<=") {
            let mut versions = vec![];
            let res = trimmed.to_string();
            versions.push(version.clone());
            versions.push(res.clone());
            versions.sort_by(|a, b| {
                let version_a = Version::parse(a);
                let version_b = Version::parse(b);
                match (version_a, version_b) {
                    (Ok(v_a), Ok(v_b)) => v_b.cmp(&v_a),
                    (Ok(_), Err(_)) => Ordering::Less,
                    (Err(_), Ok(_)) => Ordering::Greater,
                    (Err(_), Err(_)) => Ordering::Equal,
                }
            });
            if versions[1].clone() == version.clone() {
                matched = true;
            }
        }
        Ok(matched)
    }

    /// 检查一个版本 version 是否匹配一个版本匹配表达式 patched 中的任意一个规则
    pub async fn match_version(&self, patched: String, version: String) -> Result<bool, Error> {
        let mut matched = false;
        let mut part_petched = vec![];
        let parts: Vec<&str> = patched.split('|').collect();
        for part in parts {
            part_petched.push(part);
        }
        for np in part_petched {
            let oneline_patched = np.to_string();
            if oneline_patched.clone().contains(",") {
                //closed interval
                matched = matched
                    || self
                        .process_closed_interval_of_match_version(
                            oneline_patched.clone(),
                            version.clone(),
                        )
                        .await
                        .unwrap();
            } else if oneline_patched.clone().contains("^") {
                //specific version
                if let Some(trimmed) = oneline_patched.strip_prefix("^") {
                    let res = trimmed.to_string();
                    if version == res {
                        matched = true;
                    }
                }
            } else {
                //open interval
                matched = matched
                    || self
                        .process_open_interval_of_match_version(
                            oneline_patched.clone(),
                            version.clone(),
                        )
                        .await?;
            }
        }
        Ok(matched)
    }
    
    /// 查询并返回指定 crate 在指定版本上未修复的所有 RustSec 漏洞详情。
    pub async fn get_direct_rustsec(
        &self,
        cname: &str,
        version: &str,
    ) -> Result<Vec<NewRustsec>, Error> {
        tracing::info!("enter get direct_rustsec");
        let rows = self
            .client
            .query("SELECT * FROM rustsecs;", &[])
            .await
            .unwrap();
        let mut get_direct_rust_sec = vec![];
        for row in rows {
            let t_aliases: String = row.get("aliases");
            let parts: Vec<&str> = t_aliases.split(';').collect();
            let mut real_aliases = vec![];
            for part in parts {
                real_aliases.push(part.to_string());
            }
            let rs = RustSec {
                id: row.get("id"),
                cratename: row.get("cratename"),
                patched: row.get("patched"),
                aliases: real_aliases.clone(),
                small_desc: row.get("small_desc"),
            };
            get_direct_rust_sec.push(rs.clone());
        }
        let mut getres = vec![];
        for rc in get_direct_rust_sec {
            if rc.cratename.clone() == *cname {
                let matched = self
                    .match_version(rc.clone().patched, version.to_string())
                    .await
                    .unwrap();
                if !matched {
                    let rows2 = self
                        .client
                        .query("SELECT * FROM rustsec_info WHERE id=$1;", &[&rc.clone().id])
                        .await
                        .unwrap();
                    for row in rows2 {
                        let tmp_id: String = row.get("id");
                        let rs_url =
                            "https://rustsec.org/advisories/".to_string() + &tmp_id + ".html";
                        let nrs = NewRustsec {
                            id: row.get("id"),
                            subtitle: row.get("subtitle"),
                            reported: row.get("reported"),
                            issued: row.get("issued"),
                            package: row.get("package"),
                            ttype: row.get("type"),
                            keywords: row.get("keywords"),
                            aliases: row.get("aliases"),
                            reference: row.get("reference"),
                            patched: row.get("patched"),
                            unaffected: row.get("unaffected"),
                            description: row.get("description"),
                            url: rs_url.clone(),
                        };
                        getres.push(nrs.clone());
                    }
                }
            }
        }
        tracing::info!("finish get direct_rustsec");
        Ok(getres)
    }
    pub async fn get_dependency_rustsec(
        &self,
        nameversion: HashSet<String>,
    ) -> Result<Vec<NewRustsec>, Error> {
        let rows = self
            .client
            .query("SELECT * FROM rustsecs;", &[])
            .await
            .unwrap();
        let mut get_all_rust_sec = vec![];
        for row in rows {
            let t_aliases: String = row.get("aliases");
            let parts: Vec<&str> = t_aliases.split(';').collect();
            let mut real_aliases = vec![];
            for part in parts {
                real_aliases.push(part.to_string());
            }
            let rs = RustSec {
                id: row.get("id"),
                cratename: row.get("cratename"),
                patched: row.get("patched"),
                aliases: real_aliases.clone(),
                small_desc: row.get("small_desc"),
            };
            get_all_rust_sec.push(rs.clone());
        }
        let mut getres = vec![];
        for nv in nameversion {
            let parts: Vec<&str> = nv.split('/').collect();
            let cname = parts[0].to_string();
            let version = parts[1].to_string();
            for rc in get_all_rust_sec.clone() {
                if rc.cratename.clone() == cname {
                    let matched = self
                        .match_version(rc.clone().patched, version.to_string())
                        .await
                        .unwrap();
                    if !matched {
                        let rows2 = self
                            .client
                            .query("SELECT * FROM rustsec_info WHERE id=$1;", &[&rc.clone().id])
                            .await
                            .unwrap();
                        for row in rows2 {
                            let tmp_id: String = row.get("id");
                            let rs_url =
                                "https://rustsec.org/advisories/".to_string() + &tmp_id + ".html";
                            let nrs = NewRustsec {
                                id: row.get("id"),
                                subtitle: row.get("subtitle"),
                                reported: row.get("reported"),
                                issued: row.get("issued"),
                                package: row.get("package"),
                                ttype: row.get("type"),
                                keywords: row.get("keywords"),
                                aliases: row.get("aliases"),
                                reference: row.get("reference"),
                                patched: row.get("patched"),
                                unaffected: row.get("unaffected"),
                                description: row.get("description"),
                                url: rs_url.clone(),
                            };
                            getres.push(nrs.clone());
                        }
                        //getres.push(rc.clone());
                    }
                }
            }
        }
        let unique: Vec<NewRustsec> = getres
            .into_iter()
            .collect::<HashSet<_>>()
            .into_iter()
            .collect();
        Ok(unique)
    }
    /*#[allow(dead_code)]
    pub async fn get_direct_cve_by_cratenameandversion(
        &self,
        cratename: &str,
        version: &str,
    ) -> Result<Vec<String>, Error> {
        let rows = self.client.query("SELECT * FROM cves;", &[]).await.unwrap();
        let mut getallcves = vec![];
        for row in rows {
            let cveinfo = CveInfo {
                cve_id: row.get("cve_id"),
                url: "".to_string(),
                description: row.get("description"),
                crate_name: row.get("name"),
                start_version: row.get("start_version"),
                end_version: row.get("end_version"),
            };
            getallcves.push(cveinfo);
        }
        let mut getres = vec![];
        for cveinfo in getallcves {
            let mut version3 = vec![];
            if cveinfo.crate_name.clone() == *cratename {
                version3.push(cveinfo.start_version.clone());
                version3.push(cveinfo.end_version.clone());
                version3.push(version.to_string());
                version3.sort_by(|a, b| {
                    let version_a = Version::parse(a);
                    let version_b = Version::parse(b);

                    match (version_a, version_b) {
                        (Ok(v_a), Ok(v_b)) => v_b.cmp(&v_a), // 从高到低排序
                        (Ok(_), Err(_)) => Ordering::Less,   // 无法解析的版本号认为更小
                        (Err(_), Ok(_)) => Ordering::Greater,
                        (Err(_), Err(_)) => Ordering::Equal,
                    }
                });
                if version3[1].clone() == *version {
                    getres.push(cveinfo.cve_id.clone());
                }
            }
        }

        Ok(getres)
    }
    #[allow(dead_code)]
    pub async fn get_dependency_cve_by_cratenameandversion(
        &self,
        nameversion: HashSet<String>,
    ) -> Result<Vec<String>, Error> {
        let rows = self.client.query("SELECT * FROM cves;", &[]).await.unwrap();
        let mut getallcves = vec![];
        for row in rows {
            let cveinfo = CveInfo {
                cve_id: row.get("cve_id"),
                url: "".to_string(),
                description: row.get("description"),
                crate_name: row.get("name"),
                start_version: row.get("start_version"),
                end_version: row.get("end_version"),
            };
            getallcves.push(cveinfo);
        }
        let mut getres = vec![];
        for nv in nameversion {
            let parts: Vec<&str> = nv.split('/').collect();
            let cratename = parts[0].to_string();
            let crateversion = parts[1].to_string();
            for cveinfo in getallcves.clone() {
                let mut version3 = vec![];
                if cveinfo.crate_name.clone() == *cratename {
                    version3.push(cveinfo.start_version.clone());
                    version3.push(cveinfo.end_version.clone());
                    version3.push(crateversion.to_string());
                    version3.sort_by(|a, b| {
                        let version_a = Version::parse(a);
                        let version_b = Version::parse(b);

                        match (version_a, version_b) {
                            (Ok(v_a), Ok(v_b)) => v_b.cmp(&v_a), // 从高到低排序
                            (Ok(_), Err(_)) => Ordering::Less,   // 无法解析的版本号认为更小
                            (Err(_), Ok(_)) => Ordering::Greater,
                            (Err(_), Err(_)) => Ordering::Equal,
                        }
                    });
                    if version3[1].clone() == *crateversion {
                        getres.push(cveinfo.cve_id.clone());
                    }
                }
            }
        }
        let unique: Vec<String> = getres
            .into_iter()
            .collect::<HashSet<_>>()
            .into_iter()
            .collect();
        Ok(unique)
    }*/

    /// 根据程序的命名空间（namespace）和名称（name）从数据库中查询其对应的许可证（license），并返回所有查到的许可证字符串组成的列表。
    pub async fn get_license_by_name(
        &self,
        namespace: &str,
        name: &str,
    ) -> Result<Vec<String>, Error> {
        let rows = self
            .client
            .query(
                "SELECT license FROM license WHERE program_namespace = $1 and program_name = $2;",
                &[&namespace.to_string(), &name.to_string()],
            )
            .await
            .unwrap();
        let mut licenses = vec![];
        for row in rows {
            let new_license: String = row.get(0);
            licenses.push(new_license);
        }
        licenses.push("None".to_string());
        Ok(licenses)
    }

    /// 从 PostgreSQL 数据库中查询并返回指定 crate 的详细信息。
    pub async fn query_crates_info_from_pg(
        &self,
        id: &str,
        name: String,
    ) -> Result<Vec<Crateinfo>, Box<dyn std::error::Error>> {
        tracing::info!("start query crates from pg");
        let rows = self
            .client
            .query(
                "SELECT * FROM crates_info WHERE id = $1;",
                &[&id.to_string()],
            )
            .await
            .unwrap();

        let mut cf = vec![];
        for row in rows {
            let desc: String = row.get("description");
            let dcyct: i32 = row.get("direct_dependency");
            let indcyct: i32 = row.get("indirect_dependency");
            let dtct: i32 = row.get("direct_dependent");
            let indtct: i32 = row.get("indirect_dependent");
            let cs: String = row.get("cves");
            let vs: String = row.get("versions");
            let lcs: String = row.get("license");
            let gu: String = row.get("github_url");
            let du: String = row.get("doc_url");
            let dep_cs: String = row.get("dep_cves");
            let mut getcves = vec![];
            let everypartscs: Vec<&str> = cs.split("||||||").collect();
            for part in everypartscs {
                let new_part = part.to_string();
                let parts2: Vec<&str> = new_part.split("------").collect();
                if parts2.len() == 13 {
                    let onecve = NewRustsec {
                        id: parts2[0].to_string(),
                        subtitle: parts2[1].to_string(),
                        reported: parts2[2].to_string(),
                        issued: parts2[3].to_string(),
                        package: parts2[4].to_string(),
                        ttype: parts2[5].to_string(),
                        keywords: parts2[6].to_string(),
                        aliases: parts2[7].to_string(),
                        reference: parts2[8].to_string(),
                        patched: parts2[9].to_string(),
                        unaffected: parts2[10].to_string(),
                        description: parts2[12].to_string(),
                        url: parts2[11].to_string(),
                    };
                    getcves.push(onecve);
                }
            }

            let mut getdepcs = vec![];
            let everypartsdepcs: Vec<&str> = dep_cs.split("||||||").collect();
            for part in everypartsdepcs {
                let new_part = part.to_string();
                let parts2: Vec<&str> = new_part.split("------").collect();
                if parts2.len() == 13 {
                    let onecve = NewRustsec {
                        id: parts2[0].to_string(),
                        subtitle: parts2[1].to_string(),
                        reported: parts2[2].to_string(),
                        issued: parts2[3].to_string(),
                        package: parts2[4].to_string(),
                        ttype: parts2[5].to_string(),
                        keywords: parts2[6].to_string(),
                        aliases: parts2[7].to_string(),
                        reference: parts2[8].to_string(),
                        patched: parts2[9].to_string(),
                        unaffected: parts2[10].to_string(),
                        description: parts2[12].to_string(),
                        url: parts2[11].to_string(),
                    };
                    getdepcs.push(onecve);
                }
            }

            let mut getversions = vec![];
            let partsvs: Vec<&str> = vs.split('/').collect();
            for part in partsvs {
                getversions.push(part.to_string());
            }
            let res_crates_info = Crateinfo {
                crate_name: name.clone(),
                description: desc.clone(),
                dependencies: DependencyCount {
                    direct: dcyct as usize,
                    indirect: indcyct as usize,
                },
                dependents: DependentCount {
                    direct: dtct as usize,
                    indirect: indtct as usize,
                },
                cves: getcves,
                license: lcs.clone(),
                github_url: gu.clone(),
                doc_url: du.clone(),
                versions: getversions,
                dep_cves: getdepcs,
            };
            cf.push(res_crates_info);
        }
        Ok(cf)
    }

    ///将一组 CVE 逐个处理，提取关键字段，进行补全空值
    pub async fn process_cves(
        &self,
        cves: Vec<NewRustsec>,
    ) -> Result<String, Box<dyn std::error::Error>> {
        let mut every_cs = vec![];
        for rs in cves {
            let t_id = rs.clone().id;
            let mut t_subtitle = rs.clone().subtitle;
            if t_subtitle.is_empty() {
                t_subtitle = "Null".to_string();
            }
            let mut t_reported = rs.clone().reported;
            if t_reported.is_empty() {
                t_reported = "Null".to_string();
            }
            let mut t_issued = rs.clone().issued;
            if t_issued.is_empty() {
                t_issued = "Null".to_string();
            }
            let mut t_package = rs.clone().package;
            if t_package.is_empty() {
                t_package = "Null".to_string();
            }
            let mut t_type = rs.clone().ttype;
            if t_type.is_empty() {
                t_type = "Null".to_string();
            }
            let mut t_keywords = rs.clone().keywords;
            if t_keywords.is_empty() {
                t_keywords = "Null".to_string();
            }
            let mut t_aliases = rs.clone().aliases;
            if t_aliases.is_empty() {
                t_aliases = "Null".to_string();
            }
            let mut t_reference = rs.clone().reference;
            if t_reference.is_empty() {
                t_reference = "Null".to_string();
            }
            let mut t_patched = rs.clone().patched;
            if t_patched.is_empty() {
                t_patched = "Null".to_string();
            }
            let mut t_unaffected = rs.clone().unaffected;
            if t_unaffected.is_empty() {
                t_unaffected = "Null".to_string();
            }
            let mut t_desc = rs.clone().description;
            if t_desc.is_empty() {
                t_desc = "Null".to_string();
            }
            let t_url = rs.clone().url;
            let tmp_strings = [
                t_id,
                t_subtitle,
                t_reported,
                t_issued,
                t_package,
                t_type,
                t_keywords,
                t_aliases,
                t_reference,
                t_patched,
                t_unaffected,
                t_url,
                t_desc,
            ];
            let result: String = tmp_strings
                .iter()
                .filter(|&s| !s.is_empty())
                .cloned() // 复制引用的字符串
                .collect::<Vec<String>>()
                .join("------");
            every_cs.push(result);
        }
        let cs = every_cs.clone().join("||||||");
        Ok(cs)
    }
    
    /// 该函数异步将指定 crate 的信息（描述、依赖、漏洞、版本、许可证等）插入 PostgreSQL 数据库表。
    pub async fn insert_crates_info_into_pg(
        &self,
        crateinfo: Crateinfo,
        namespace: String,
        name: String,
        version: String,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let id = namespace.clone() + "/" + &name + "/" + &version;
        let dcyct = crateinfo.dependencies.direct as i32;
        let indcyct = crateinfo.dependencies.indirect as i32;
        let dtct = crateinfo.dependents.direct as i32;
        let indtct = crateinfo.dependents.indirect as i32;
        let vs = crateinfo.versions.clone().join("/");
        let cs = self.process_cves(crateinfo.clone().cves).await.unwrap();
        let depcs = self.process_cves(crateinfo.clone().dep_cves).await.unwrap();
        self.client
            .execute(
                "
                        INSERT INTO crates_info (
                            id,description,direct_dependency,indirect_dependency,
                            direct_dependent,indirect_dependent,cves,dep_cves,versions,
                            license,github_url,doc_url
                        ) VALUES ($1, $2, $3, $4, $5, $6, $7,$8,$9,$10,$11,$12);
                        ",
                &[
                    &id,
                    &crateinfo.description,
                    &dcyct,
                    &indcyct,
                    &dtct,
                    &indtct,
                    &cs,
                    &depcs,
                    &vs,
                    &crateinfo.license,
                    &crateinfo.github_url,
                    &crateinfo.doc_url,
                ],
            )
            .await
            .unwrap();
        Ok(())
    }

    /// 该函数异步根据命名空间前后缀、包名和版本号，从数据库中查询对应的图结构信息，并返回图字符串列表。
    pub async fn get_graph_from_pg(
        &self,
        nsfront: String,
        nsbehind: String,
        name: String,
        version: String,
    ) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        let id = nsfront + "/" + &nsbehind + "/" + &name + "/" + &version;
        let rows = self
            .client
            .query("SELECT * FROM graph_info WHERE id = $1;", &[&id])
            .await
            .unwrap();
        let mut res = vec![];
        for row in rows {
            let graph: String = row.get("graph");
            res.push(graph);
        }
        Ok(res)
    }

    ///该函数异步将指定的命名空间、包名、版本号组合成唯一 ID，并将对应的图结构字符串插入到数据库的 `graph_info` 表中。

    pub async fn insert_graph_into_pg(
        &self,
        nsfront: String,
        nsbehind: String,
        name: String,
        version: String,
        graph: String,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let id = nsfront + "/" + &nsbehind + "/" + &name + "/" + &version;
        self.client
            .execute(
                "
                        INSERT INTO graph_info (
                            id,graph
                        ) VALUES ($1, $2);
                        ",
                &[&id, &graph],
            )
            .await
            .unwrap();
        Ok(())
    }

    ///该函数异步根据命名空间前后缀和包名，从数据库中查询对应的版本信息，并返回版本字符串列表。
    pub async fn get_version_from_pg(
        &self,
        nsfront: String,
        nsbehind: String,
        name: String,
    ) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        let id = nsfront + "/" + &nsbehind + "/" + &name;
        let rows = self
            .client
            .query("SELECT * FROM version_info WHERE id = $1;", &[&id])
            .await
            .unwrap();
        let mut res = vec![];
        for row in rows {
            let newversion: String = row.get("versions");
            res.push(newversion);
        }
        Ok(res)
    }

    ///该函数异步将命名空间、包名和版本相关信息组合成唯一 ID，并将多个版本详情拼接成字符串后，插入到数据库的 `version_info` 表中。
    pub async fn insert_version_into_pg(
        &self,
        nsbehind: String,
        nsfront: String,
        name: String,
        versionpg: Vec<Versionpage>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let id = nsfront + "/" + &nsbehind + "/" + &name;
        let mut every_version = vec![];
        for vp in versionpg {
            let dts_count = vp.dependents.to_string();
            let one_version = vp.version.clone()
                + "|"
                + &vp.updated_at.clone()
                + "|"
                + &vp.downloads.clone()
                + "|"
                + &dts_count;
            every_version.push(one_version);
        }
        let versions = every_version.join("/");
        self.client
            .execute(
                "
                        INSERT INTO version_info (
                            id,versions
                        ) VALUES ($1, $2);
                        ",
                &[&id, &versions],
            )
            .await
            .unwrap();
        Ok(())
    }
    ///该函数异步根据给定的包名，从数据库中查找对应的包 ID，再根据该 ID 查询其所有版本信息，找到指定版本后返回该版本的更新时间和下载量组成的字符串。
    pub async fn get_dump_from_cratesio_pg(
        &self,
        name: String,
        version: String,
    ) -> Result<String, Box<dyn std::error::Error>> {
        tracing::info!("enter get dump");
        let rows1 = self
            .client
            .query("SELECT * FROM crates WHERE name=$1 LIMIT 1", &[&name])
            .await
            .unwrap();
        tracing::info!("finish get id");
        let mut res = "".to_string();
        for row in rows1 {
            tracing::info!("enter rows1");
            let crate_id: i32 = row.get("id");
            tracing::info!("id:{}", crate_id.clone());
            tracing::info!("start get num,up,dl");
            let rows = self
                .client
                .query("SELECT * FROM versions WHERE crate_id=$1;", &[&crate_id])
                .await
                .unwrap();
            tracing::info!("finish get num,up,dl");
            for row in rows {
                let num: String = row.get("num");
                let updated_at: NaiveDateTime = row.get("updated_at");
                let downloads: i32 = row.get("downloads");
                let downloads_string = downloads.to_string();
                let updated_at_string = updated_at.to_string();
                if num == version {
                    res = updated_at_string + "/" + &downloads_string;
                }
            }
            tracing::info!("finish get dump");
        }
        Ok(res)
    }
    ///该函数异步根据命名空间、包名和版本号，从数据库查询依赖缓存，解析依赖字符串生成依赖详情列表，
    /// 返回包含直接和间接依赖计数及依赖数据的结构体数组。
    pub async fn get_dependency_from_pg(
        &self,
        nsfront: String,
        nsbehind: String,
        name: String,
        version: String,
    ) -> Result<Vec<DependencyInfo>, Box<dyn std::error::Error>> {
        let id = nsfront.clone() + "/" + &nsbehind + "/" + &name + "/" + &version;
        let rows = self
            .client
            .query("SELECT * FROM dependency_cache WHERE id = $1;", &[&id])
            .await
            .unwrap();
        let mut res = vec![];
        for row in rows {
            let all_dependency: String = row.get("dependency");
            let direct: i32 = row.get("direct_count");
            let indirect: i32 = row.get("indirect_count");
            let mut deps = vec![];
            let parts1: Vec<&str> = all_dependency.split("|").collect();
            for part in parts1 {
                let one_dep = part.to_string();
                let parts2: Vec<&str> = one_dep.split("/").collect();
                if parts2.len() == 5 {
                    let dcs = parts2[4].to_string();
                    let dcc = dcs.parse::<usize>().unwrap();
                    let one_res = DependencyCrateInfo {
                        crate_name: parts2[0].to_string(),
                        version: parts2[1].to_string(),
                        relation: parts2[2].to_string(),
                        license: parts2[3].to_string(),
                        dependencies: dcc,
                    };
                    deps.push(one_res);
                }
            }
            let real_res = DependencyInfo {
                direct_count: direct as usize,
                indirect_count: indirect as usize,
                data: deps,
            };
            res.push(real_res);
        }
        Ok(res)
    }

    ///该函数异步根据命名空间、包名和版本号，从数据库查询被依赖缓存，
    /// 解析被依赖字符串生成被依赖详情列表，
    /// 返回包含直接和间接被依赖计数及相关数据的结构体数组。
    pub async fn get_dependent_from_pg(
        &self,
        nsfront: String,
        nsbehind: String,
        name: String,
        version: String,
    ) -> Result<Vec<DependentInfo>, Box<dyn std::error::Error>> {
        let id = nsfront.clone() + "/" + &nsbehind + "/" + &name + "/" + &version;
        let rows = self
            .client
            .query("SELECT * FROM dependent_cache WHERE id = $1;", &[&id])
            .await
            .unwrap();
        let mut res = vec![];
        for row in rows {
            let all_dependent: String = row.get("dependent");
            let direct: i32 = row.get("direct_count");
            let indirect: i32 = row.get("indirect_count");
            let mut deps = vec![];
            let parts1: Vec<&str> = all_dependent.split("|").collect();
            for part in parts1 {
                let one_dep = part.to_string();
                let parts2: Vec<&str> = one_dep.split("/").collect();
                if parts2.len() == 3 {
                    let one_res = DependentData {
                        crate_name: parts2[0].to_string(),
                        version: parts2[1].to_string(),
                        relation: parts2[2].to_string(),
                    };
                    deps.push(one_res);
                }
            }
            let real_res = DependentInfo {
                direct_count: direct as usize,
                indirect_count: indirect as usize,
                data: deps,
            };
            res.push(real_res);
        }
        Ok(res)
    }
    ///该函数异步将命名空间、包名、版本号与依赖信息组合成唯一 ID，
    /// 将依赖详情拼接成字符串，并插入到数据库的 `dependency_cache` 表中。
    pub async fn insert_dependency_into_pg(
        &self,
        nsfront: String,
        nsbehind: String,
        name: String,
        version: String,
        dep_info: DependencyInfo,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let id = nsfront.clone() + "/" + &nsbehind + "/" + &name + "/" + &version;
        let mut every_dep = vec![];
        for one_dep in dep_info.data {
            let dcs = one_dep.dependencies.to_string();
            let one_res = one_dep.crate_name.clone()
                + "/"
                + &one_dep.version
                + "/"
                + &one_dep.relation
                + "/"
                + &one_dep.license
                + "/"
                + &dcs;
            every_dep.push(one_res);
        }
        let real_dep = every_dep.join("|");
        self.client
            .execute(
                "
                        INSERT INTO dependency_cache (
                            id,direct_count,indirect_count,dependency
                        ) VALUES ($1, $2,$3,$4);
                        ",
                &[
                    &id,
                    &(dep_info.direct_count as i32),
                    &(dep_info.indirect_count as i32),
                    &real_dep,
                ],
            )
            .await
            .unwrap();
        Ok(())
    }
    ///该函数异步将命名空间、包名、版本号组合成唯一 ID，
    /// 并将被依赖信息拼接成字符串，插入到数据库的 `dependent_cache` 表中。
    pub async fn insert_dependent_into_pg(
        &self,
        nsfront: String,
        nsbehind: String,
        name: String,
        version: String,
        dep_info: DependentInfo,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let id = nsfront.clone() + "/" + &nsbehind + "/" + &name + "/" + &version;
        let mut every_dep = vec![];
        for one_dep in dep_info.data {
            let one_res =
                one_dep.crate_name.clone() + "/" + &one_dep.version + "/" + &one_dep.relation;
            every_dep.push(one_res);
        }
        let real_dep = every_dep.join("|");
        self.client
            .execute(
                "
                        INSERT INTO dependent_cache (
                            id,direct_count,indirect_count,dependent
                        ) VALUES ($1, $2,$3,$4);
                        ",
                &[
                    &id,
                    &(dep_info.direct_count as i32),
                    &(dep_info.indirect_count as i32),
                    &real_dep,
                ],
            )
            .await
            .unwrap();
        Ok(())
    }
    /// 该函数异步将用户信息（包括邮箱、头像、姓名和过期时间）
    /// 插入或更新到数据库的 `userloginfo` 表中。
    pub async fn insert_userinfo_into_pg(
        &self,
        info: Userinfo,
    ) -> Result<(), Box<dyn std::error::Error>> {
        tracing::info!("enter insert userinfo into pg");
        self.client
            .execute(
                "INSERT INTO userloginfo(
                        id,image,name,expires) VALUES ($1, $2,$3,$4)
                        ON CONFLICT (id)
                        DO UPDATE SET image = $2, name = $3, expires = $4;",
                &[
                    &info.user.email,
                    &info.user.image,
                    &info.user.name,
                    &info.expires,
                ],
            )
            .await
            .unwrap();
        Ok(())
    }
    ///该函数异步根据用户邮箱，从数据库查询该用户上传的所有包信息，
    /// 并返回包名和上传时间的列表。
    pub async fn query_uploaded_crates_from_pg(
        &self,
        email: String,
    ) -> Result<Vec<UploadedCrate>, Box<dyn std::error::Error>> {
        let rows = self
            .client
            .query("SELECT * FROM uploadedcrate WHERE email=$1", &[&email])
            .await
            .unwrap();
        let mut res = vec![];
        for row in rows {
            let name: String = row.get("filename");
            let time: String = row.get("uploadtime");
            let tmp_res = UploadedCrate { name, time };
            res.push(tmp_res);
        }
        Ok(res)
    }
    ///该函数异步根据用户邮箱，从数据库查询该用户上传的所有
    ///  GitHub URL 及上传时间，并返回列表。

    pub async fn query_uploaded_url_from_pg(
        &self,
        email: String,
    ) -> Result<Vec<UploadedCrate>, Box<dyn std::error::Error>> {
        let rows = self
            .client
            .query("SELECT * FROM uploadedurl WHERE email=$1", &[&email])
            .await
            .unwrap();
        let mut res = vec![];
        for row in rows {
            let name: String = row.get("githuburl");
            let time: String = row.get("uploadtime");
            let tmp_res = UploadedCrate { name, time };
            res.push(tmp_res);
        }
        Ok(res)
    }
    /// 该函数异步将敏感泄露检测结果根据唯一 ID 插入或更新到数据库的
    ///  `senseleak_res` 表中。
    pub async fn insert_sensleak_result_into_pg(
        &self,
        id: String,
        result: String,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.client
            .execute(
                "INSERT INTO senseleak_res(
                        id,res) VALUES ($1, $2)
                        ON CONFLICT (id)
                        DO UPDATE SET res=$2;",
                &[&id, &result],
            )
            .await
            .unwrap();
        Ok(())
    }
    ///该函数异步将镜像检查结果根据唯一 ID 插入或更新到
    /// 数据库的 `mirchecker_res` 表中。
    pub async fn insert_mirchecker_result_into_pg(
        &self,
        id: String,
        result: String,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.client
            .execute(
                "INSERT INTO mirchecker_res(
                        id,res) VALUES ($1, $2)
                        ON CONFLICT (id)
                        DO UPDATE SET res=$2;",
                &[&id, &result],
            )
            .await
            .unwrap();
        Ok(())
    }
    /// 该函数异步将镜像检查失败的唯一 
    /// ID 插入到数据库的 `mirchecker_run_failed` 表中，
    /// 若已存在则不做任何操作。
    pub async fn insert_mirchecker_failed_into_pg(
        &self,
        id: String,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.client
            .execute(
                "INSERT INTO mirchecker_run_failed(
                        id) VALUES ($1)
                        ON CONFLICT (id)
                        DO NOTHING;",
                &[&id],
            )
            .await
            .unwrap();
        Ok(())
    }
    ///该函数异步根据唯一 ID，从数据库中查询敏感泄露检测结果，
    /// 返回结果字符串；若无结果则返回空的 JSON 数组字符串 `"[]"`。
    #[allow(clippy::len_zero)]
    pub async fn get_senseleak_from_pg(
        &self,
        id: String,
    ) -> Result<String, Box<dyn std::error::Error>> {
        let rows = self
            .client
            .query("SELECT * FROM senseleak_res WHERE id=$1", &[&id])
            .await
            .unwrap();
        let mut tmp_res = vec![];
        for row in rows {
            let s_res: String = row.get("res");
            tmp_res.push(s_res);
        }
        let mut real_res = "[]".to_string();
        if tmp_res.len() != 0 {
            real_res = tmp_res[0].clone();
        }
        Ok(real_res)
    }
    ///该函数异步根据唯一 ID，从数据库查询镜像检查结果，
    /// 返回结果字符串；如果没有结果则返回空字符串。
    #[allow(clippy::len_zero)]
    pub async fn get_mirchecker_from_pg(
        &self,
        id: String,
    ) -> Result<String, Box<dyn std::error::Error>> {
        let rows = self
            .client
            .query("SELECT * FROM mirchecker_res WHERE id=$1", &[&id])
            .await
            .unwrap();
        let mut tmp_res = vec![];
        for row in rows {
            let s_res: String = row.get("res");
            tmp_res.push(s_res);
        }
        let mut real_res = "".to_string();
        if tmp_res.len() != 0 {
            real_res = tmp_res[0].clone();
        }
        Ok(real_res)
    }
    ///该函数异步根据唯一 ID 查询镜像检查失败记录表，返回布尔值表示该 
    /// ID 是否未失败（存在记录返回 `false`，否则返回 `true`）。
    #[allow(clippy::len_zero)]
    pub async fn get_mirchecker_run_state_from_pg(
        &self,
        id: String,
    ) -> Result<bool, Box<dyn std::error::Error>> {
        let rows = self
            .client
            .query("SELECT * FROM mirchecker_run_failed WHERE id=$1", &[&id])
            .await
            .unwrap();
        let mut tmp_res = vec![];
        for row in rows {
            let s_res: String = row.get("id");
            tmp_res.push(s_res);
        }
        let mut real_res = true;
        if tmp_res.len() != 0 {
            real_res = false;
        }
        Ok(real_res)
    }
}
