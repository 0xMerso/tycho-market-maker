// I have 2 Rust program
// The first is the market maker program (MM)
// The second is a monitoring of what the MM does (moni)
// I want to notify locally from the MM to the second (moni) that an action occurred (trade attempt, failed, succeeded, etc)
// With full of metadata context
// And also other data at a given timeframe (block time, like every 12s)

// Illustrate which tool to use for that. One idea is using Redis pub/sub stream, but I'm open to any tool
// The moni program should be able to receive multiple, simultaneous, data from each instance of the market maker bot
// Give the full code, for the producers, and the consumer
// Set comments to be explicit

// Give generics function with <T> to interact with redis stream
// Isolate redis function to have a clean code

trait RedisTrait {
    fn connect(client_options: RedicClientOptions) -> Result<Redis, Box<dyn std::error::Error>>;
}

pub struct RedicClientOptions {
    pub connection_address: String,
    pub port: i64,
}

pub struct Redis {
    pub client: redis::Client,
}

impl RedisTrait for Redis {
    fn connect(client_options: RedicClientOptions) -> Result<Redis, Box<dyn std::error::Error>> {
        let redis_url: String = format!("redis://{}:{}", client_options.connection_address, client_options.port);

        match redis::Client::open(redis_url.as_str()) {
            Ok(client) => Ok(Redis { client }),
            Err(e) => Err(Box::new(e)),
        }
    }
}

fn main() {
    dotenv::dotenv().ok();
    let port = std::env::var("REDIS_PORT").unwrap().parse::<i64>().unwrap();
    let address = std::env::var("REDIS_ADDRESS").unwrap();
    let redis_config = RedicClientOptions {
        connection_address: address.to_string(),
        port,
    };
    println!("redis_config: {:?}", redis_config.connection_address);
    match Redis::connect(redis_config) {
        Ok(client) => match client.client.get_connection() {
            Ok(mut conn) => {
                let mut pubsub = conn.as_pubsub();

                println!("pubsub");
                match pubsub.subscribe("channel") {
                    Ok(_pubs) => loop {
                        println!("pubsub loop");
                        match pubsub.get_message() {
                            Ok(msg) => match msg.get_payload::<String>() {
                                Ok(payload) => {
                                    println!("000");
                                    println!("Message Received : {:?}", payload.parse::<String>().unwrap());
                                }
                                Err(e) => {
                                    println!("111");
                                    println!("Error while getting payload : {:?}", e.to_string());
                                }
                            },
                            Err(e) => {
                                println!("Error {:?}", e.to_string())
                            }
                        }
                    },
                    Err(e) => {
                        println!("{:?}", e.to_string())
                    }
                }
            }
            Err(e) => {
                println!("Error while getting connection {:?}", e.to_string());
            }
        },
        Err(e) => {
            println!("{:?}", e.to_string())
        }
    }
}
