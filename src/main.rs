use chrono::{DateTime, Utc};
use std::time::SystemTime;
use structopt::StructOpt;
use serde::{Serialize, Deserialize};
use std::env;
use std::path::PathBuf;
use postgres::{error::Error, Client, NoTls};

////// opt ///////////

#[derive(Debug, StructOpt, Clone)]
#[structopt(name = "systime", about = "Experiments with system and db timestamps.")]
pub struct Opt {
    /// The 3 state test level bitmap in decimal.
    #[structopt(name="LEVEL", default_value="1")]
    pub level: u16,
    /// Set config-file.
    #[structopt(short = "f", long = "config-file")]
    pub config_file: Option<String>,
}

////// config //////
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ConfigFile {
    pub postgresql: Postgresql,
}

#[derive(Debug, Clone)]
pub struct Config{
    pub cf: ConfigFile,
    pub opt: Opt,
    config_path: PathBuf,
}

impl Config {
    fn new() -> Self {
        let opt = Opt::from_args();

        let config_path = Self::config_file_path(&opt)
            .expect("Couldn't get config file path");

        let config_text =
            match std::fs::read_to_string(&config_path) {
                Ok(config) => config,
                Err(e) => {
                    eprintln!("Unable to read config file {}:\n\
                        error= {:?}",
                        config_path.display(), e);
                    std::process::exit(0);
                }
            };

        let cf: ConfigFile = toml::from_str(&config_text).unwrap();
        Config {
            cf, opt, config_path
        }
    }
    fn config_file_path(opt: &Opt) -> Result<PathBuf, std::io::Error> {
        match opt.config_file {
            None => {
                let exe = env::current_exe()?;
                let dir = exe.parent().expect(
                    "Executable must be in some directory");
                let mut dir = dir.join("");
                dir.pop();
                dir.pop();
                dir.push("config.toml");
                Ok(dir)
            },
            Some(ref config_file) => {
                let path = std::fs::canonicalize(config_file);
                match path {
                    Ok(ref path) => {
                        println!("config file canonicalized path = {}",
                                path.display());
                    },
                    Err(ref e) =>
                        println!(
                            "oops got error = {:?} calling canonicalize on={}",
                            e, config_file),
                }
                path
            }
        }
    }
}

pub fn connect_db(cfg: &Config) -> Result<Client, Error> {
    let cf = &cfg.cf;
    let connect_str =
            format!(
                "postgres://{}{}{}@{}{}{}{}{}",
                cf.postgresql.username,
                if cf.postgresql.password.is_empty() { "" } else { ":" },
                cf.postgresql.password,
                cf.postgresql.host,
                if cf.postgresql.port.is_empty() { "" } else { ":" },
                cf.postgresql.port,
                if cf.postgresql.database.is_empty() { "" } else { "/" },
                cf.postgresql.database
            );
    Client::connect(&connect_str, NoTls)
}


#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Postgresql {
    pub username: String,
    pub password: String,
    pub host: String,
    pub port: String,
    pub database: String,
}


// create.sql
// DROP TABLE IF EXISTS foo;
// CREATE TABLE foo (
//     memo varchar,
//     import_ts timestamp default now(),
//     import_tz timestamp with time zone default now()
// );
// insert into foo(memo) values('Theo is sneezy but great!');

fn main() -> Result<(), Error> {
    let cfg = Config::new();
    let level = cfg.opt.level;
    println!("level = {}", level);

    if level & 0b00001 > 0 {
        let query = "insert into foo(memo, import_ts, import_tz)
                        values($1, $2, $3)
                        returning memo, import_ts, import_tz";

        let memo: String = "Theo is cute".into();
        let import_tz =
            match DateTime::parse_from_str(
                    "961219163957+0000", "%y%m%d%H%M%S%z") {
                Ok(dt) => dt,
                Err(e) => 
                    panic!("error parsing datetime e={}", e),
            };
        let import_ts =
            match DateTime::parse_from_str(
                    "210723120000+0000", "%y%m%d%H%M%S%z") {
                Ok(dt) => dt,
                Err(e) => 
                    panic!("error parsing datetime e={}", e),
            };
        let import_ts: SystemTime = import_ts.into();

        let mut client =
            match connect_db(&cfg) {
                Ok(clnt) => clnt,
                Err(_) => panic!("no connect"),
            };

        let rows = client.query(query, &[&memo, &import_ts, &import_tz])?;
        for row in rows.iter() {
            let memo: String = row.get(0);
            // Systime required for timestamp ( without time zone )
            let import_ts: SystemTime       = row.get(1);
            let import_ts: DateTime<Utc>    = import_ts.into();
            // Datetime required for timestamp with time zone
            let import_tz: DateTime<Utc>    = row.get(2);

            println!("inserted: memo = {}, import_ts = {}, import_tz = {}",
                memo, 
                import_ts.format("%m/%d/%Y %T"),
                import_tz.format("%m/%d/%Y %T"));
        }
    }


    //////////////////////

    if level & 0b00001 > 0 {
        let query = "select memo, 
                            import_ts, 
                            import_tz
                        from foo";

        let mut client =
            match connect_db(&cfg) {
                Ok(clnt) => clnt,
                Err(_) => panic!("no connect"),
            };

        let rows = client.query(query, &[])?;
        for row in rows.iter() {
            let memo: String = row.get(0);
            // Systime required for timestamp ( without time zone )
            let import_ts: SystemTime       = row.get(1);
            let import_ts: DateTime<Utc>    = import_ts.into();
            // Datetime required for timestamp with time zone
            let import_tz: DateTime<Utc>    = row.get(2);

            println!("memo = {}, import_ts = {}, import_tz = {}",
                memo, 
                import_ts.format("%m/%d/%Y %T"),
                import_tz.format("%m/%d/%Y %T"));
        }
    }

    //////////////////////

    if level & 0b00010 > 0 {
        println!("cfg = {:?}, config_path={}", cfg, cfg.config_path.display());
    }

    if level & 0b00100 > 0 {
        //////////////////////

        let system_time = SystemTime::now();
        let datetime: DateTime<Utc> = system_time.into();
        println!("Current now() from SystemTime= {}",
                    datetime.format("%m/%d/%Y %T"));

        // take round trip from an arbitrary datetime to systemtime and back.
        //let datetime =
            DateTime::parse_from_rfc3339(
                    "1996-12-19T16:39:57-00:00")
                .unwrap();

        let datetime =
            match DateTime::parse_from_str(
                    "961219163957+0000", "%y%m%d%H%M%S%z") {
                Ok(dt) => dt,
                Err(e) => 
                    panic!("error parsing datetime e={}", e),
            };
        println!("1: Arbitrary Datetime = {}", datetime.format("%m/%d/%Y %T"));

        let back_to_systime: SystemTime = SystemTime::from(datetime);
        let back_to_datetime: DateTime<Utc> = back_to_systime.into();
        println!("1: back_to_datetime from systemtime = {}", 
            back_to_datetime.format("%m/%d/%Y %T"));


        let datetime =
            match DateTime::parse_from_str("2018-01-26T18:30:09.453Z", "%+") {
                Ok(dt) => dt,
                Err(e) => 
                    panic!("error parsing datetime e={}", e),
            };
        println!("2: Arbitrary Datetime = {}", datetime.format("%m/%d/%Y %T"));

        let back_to_systime: SystemTime = SystemTime::from(datetime);
        let back_to_datetime: DateTime<Utc> = back_to_systime.into();
        println!("2: back_to_datetime from systemtime = {}", 
            back_to_datetime.format("%m/%d/%Y %T"));
        println!("3: back_to_datetime from systemtime = {}", 
            back_to_datetime.format("%+"));
        println!("4: back_to_datetime from systemtime = {}", 
            back_to_datetime.format("%Y-%m-%dT%H:%M:%S%.f%:z"));


        let datetime =
            match DateTime::parse_from_str("2021-01-01T05:00:00.003Z", "%+") {
                Ok(dt) => dt,
                Err(e) => 
                    panic!("error parsing datetime e={}", e),
            };
        println!("2: Arbitrary Datetime = {}", datetime.format("%m/%d/%Y %T"));

        let back_to_systime: SystemTime = SystemTime::from(datetime);
        let back_to_datetime: DateTime<Utc> = back_to_systime.into();
        println!("2: back_to_datetime from systemtime = {}", 
            back_to_datetime.format("%m/%d/%Y %T"));
        println!("3: back_to_datetime from systemtime = {}", 
            back_to_datetime.format("%+"));
        println!("4: back_to_datetime from systemtime = {}", 
            back_to_datetime.format("%Y-%m-%dT%H:%M:%S%.f%:z"));

    }

    Ok(())
}
