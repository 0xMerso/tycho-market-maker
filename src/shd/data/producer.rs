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
