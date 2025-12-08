import redis
import os
import json
import asyncio

from src.docker_mcp.handlers import DockerHandlers

async def main():
    redis_host = os.environ.get("REDIS_HOST", "127.0.0.1")
    redis_port = int(os.environ.get("REDIS_PORT", 6379))

    print(f"Connecting to Redis at {redis_host}:{redis_port}...")
    r = redis.Redis(host=redis_host, port=redis_port, decode_responses=True)

    queue_key = "mcp::tasks::docker"
    print(f"Listening for Docker tasks on '{queue_key}'")

    while True:
        try:
            # blpop returns a tuple: (queue_name, task_json)
            _, task_json = r.blpop([queue_key])
            print(f"Received Docker task: {task_json}")

            task = json.loads(task_json)
            result_key = f"mcp::result::{task['id']}"

            # Placeholder for task execution
            payload = {
                "status": "pending",
                "result": "Task received by Python listener, processing not yet implemented."
            }

            r.set(result_key, json.dumps(payload))
            print(f"Reported placeholder result to {result_key}")

        except redis.exceptions.ConnectionError as e:
            print(f"Redis connection error: {e}")
            await asyncio.sleep(5)
        except Exception as e:
            print(f"An error occurred: {e}")

if __name__ == "__main__":
    asyncio.run(main())
