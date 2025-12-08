use bollard::Docker;
use serde::{Deserialize, Serialize};
use std::fs::OpenOptions;
use std::io::Write;
use std::thread;
use std::time::Duration;

#[derive(Serialize, Deserialize, Debug)]
struct DockerTask {
    id: String,
    action: String,
    params: serde_json::Value,
}

fn log(msg: &str) {
    println!("{}", msg);
    if let Ok(mut file) = OpenOptions::new().create(true).write(true).append(true).open("docker-mcp.log") {
        writeln!(file, "{}", msg).ok();
        file.flush().ok();
    }
}

fn main() {
    log("--- PANIC-PROOF RUN ---");
    
    let redis_host = "192.168.1.93";
    let redis_url = format!("redis://{}:6379/", redis_host);
    
    // Setup connection logic (simplified for robustness)
    let client = match redis::Client::open(redis_url.clone()) {
        Ok(c) => c,
        Err(e) => { log(&format!("FATAL: Client creation failed: {}", e)); return; }
    };
    
    let mut conn = match client.get_connection() {
        Ok(c) => c,
        Err(e) => { log(&format!("FATAL: Connection failed: {}", e)); return; }
    };

    let queue_key = "mcp::tasks::docker";
    log("Entering Bulletproof Loop...");

    loop {
        // 1. Safe Pop
        let pop_result: redis::RedisResult<Option<String>> = redis::cmd("LPOP").arg(queue_key).query(&mut conn);

        match pop_result {
            Ok(Some(json_str)) => {
                log(&format!(">>> RECEIVED: {}", json_str));

                // 2. Safe Parse
                match serde_json::from_str::<DockerTask>(&json_str) {
                    Ok(task) => {
                        log(&format!("Processing Task ID: {}", task.id));
                        // Mock processing success
                        let res_key = format!("mcp::result::{}", task.id);
                        let _: () = redis::cmd("SET").arg(res_key).arg("Success").query(&mut conn).unwrap_or(());
                        log("Result written to Redis.");
                    },
                    Err(e) => log(&format!("JSON Parse Error: {}", e)),
                }
            },
            Ok(None) => {
                // Queue empty, stay silent or log sparingly
            },
            Err(e) => {
                log(&format!("Redis Error in Loop: {:?}", e));
                // Try to reconnect? For now just sleep.
            }
        }

        thread::sleep(Duration::from_secs(1));
    }
}