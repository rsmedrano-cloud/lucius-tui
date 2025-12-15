
use redis::{Client, Commands};
use serde_json::json;

fn main() {
    let client = Client::open("redis://localhost/").unwrap();
    let mut con = client.get_connection().unwrap();

    let task = json!({
        "id": "123",
        "action": "ps",
        "params": {}
    });

    let _: () = con.rpush("mcp::tasks::docker", task.to_string()).unwrap();

    println!("Task submitted!");

    let result: Vec<String> = con.blpop("mcp::result::123", 30.0).unwrap();

    println!("Result: {:?}", result);
}
