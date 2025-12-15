# Dangerous Unwrap Review

Review each item and mark with [x] to fix, or [ ] to skip.
Edit the proposed fix or commit message as needed.

---

## CRITICAL (will panic if condition not met)

### 1. SECRET_PATH env var
- [x] Fix
- **File:** `src/maker.rs:123`
- **Code:** `std::env::var("SECRET_PATH").unwrap()`
- **Risk:** Panics if env var not set
- **Proposed fix:** Use `unwrap_or_else` with error message and exit
- **Commit:** `fix(maker): handle missing SECRET_PATH env var gracefully`

### 2. EnvConfig env var unwraps
- [x] Fix
- **File:** `src/shd/types/config.rs:92-96`
- **Code:**
```rust
path: std::env::var("CONFIG_PATH").unwrap(),
testing: std::env::var("TESTING").unwrap() == "true",
heartbeat: std::env::var("HEARTBEAT").unwrap(),
wallet_private_key: std::env::var("WALLET_PRIVATE_KEY").unwrap(),
tycho_api_key: std::env::var("TYCHO_API_KEY").unwrap(),
```
- **Risk:** Panics if any env var not set
- **Proposed fix:** Return `Result` from `EnvConfig::new()` or use defaults where sensible
- **Commit:** `fix(config): handle missing env vars with clear error messages`

### 3. File operations in misc.rs
- [x] Fix
- **File:** `src/shd/utils/misc.rs:34-37`
- **Code:**
```rust
let mut file = File::open(file).unwrap();
file.read_to_string(&mut buffer).unwrap();
serde_json::from_str(&buffer).unwrap()
```
- **Risk:** Panics if file doesn't exist or invalid JSON
- **Proposed fix:** Return `Result` instead of panicking
- **Commit:** `fix(misc): return Result from file read operations`

### 4. Double unwrap on block fetch
- [x] Fix
- **File:** `src/shd/maker/impl.rs:207`
- **Code:** `provider.get_block_by_number(...).await.unwrap().unwrap()`
- **Risk:** Panics if RPC fails or block not found
- **Proposed fix:** Use `?` or handle error with continue/retry
- **Commit:** `fix(impl): handle block fetch failure gracefully`

### 5. Chain lookup unwrap
- [ ] Fix
- **File:** `src/shd/maker/impl.rs:679`
- **Code:** `crate::maker::tycho::chain(...).unwrap()`
- **Risk:** Panics for unknown network name
- **Proposed fix:** Return error or skip
- **Commit:** `fix(impl): handle unknown chain gracefully`

### 6. Token find unwraps in routing
- [ ] Fix
- **File:** `src/shd/opti/routing.rs:89-90`
- **Code:**
```rust
let base = atks.iter().find(...).unwrap().clone();
let quote = atks.iter().find(...).unwrap().clone();
```
- **Risk:** Panics if token not found in list
- **Proposed fix:** Use `ok_or` and propagate error, or skip route
- **Commit:** `fix(routing): handle missing tokens gracefully`

### 7. orders.first().unwrap()
- [ ] Fix
- **File:** `src/shd/maker/impl.rs:1023`
- **Code:** `orders.first().unwrap().clone()`
- **Risk:** Panics if orders vec is empty
- **Proposed fix:** Check if empty first or use `if let Some`
- **Commit:** `fix(impl): handle empty orders list`

---

## HIGH (edge cases can trigger panic)

### 8. Address parsing in evm.rs
- [ ] Fix
- **File:** `src/shd/utils/evm.rs:60,62,82,83,103,111,147,158`
- **Code:** Multiple `.parse().unwrap()` on addresses
- **Risk:** Panics on malformed addresses
- **Proposed fix:** Return `Result` from these functions
- **Commit:** `fix(evm): return Result for address parsing`

### 9. URL parsing unwraps
- [ ] Fix
- **File:** `src/shd/utils/evm.rs:97`, `src/shd/maker/exec/mod.rs:123,226`, `src/shd/maker/feed.rs:96`
- **Code:** `.parse::<url::Url>().unwrap()`
- **Risk:** Panics on malformed RPC URL
- **Proposed fix:** Validate URL at config load time, or return Result
- **Commit:** `fix: validate RPC URL at startup`

### 10. Price parsing in feed.rs
- [ ] Fix
- **File:** `src/shd/maker/feed.rs:106`
- **Code:** `price.to_string().parse::<u128>().unwrap()`
- **Risk:** Panics if price doesn't fit in u128
- **Proposed fix:** Handle parse error
- **Commit:** `fix(feed): handle price parsing failure`

### 11. Address parsing in tycho.rs
- [ ] Fix
- **File:** `src/shd/maker/tycho.rs:168`
- **Code:** `Bytes::from_str(...).unwrap()`
- **Risk:** Panics on invalid address
- **Proposed fix:** Filter out invalid addresses
- **Commit:** `fix(tycho): skip invalid addresses instead of panicking`

### 12. Address parsing in tycho types
- [ ] Fix
- **File:** `src/shd/types/tycho.rs:144`
- **Code:** `Bytes::from_str(serialized.address...).unwrap()`
- **Risk:** Panics on invalid address
- **Proposed fix:** Return Result or Option
- **Commit:** `fix(types): handle invalid address in deserialization`

---

## MEDIUM (less likely but possible)

### 13. time.elapsed() unwrap
- [ ] Fix
- **File:** `src/shd/data/helpers.rs:102,160`
- **Code:** `time.elapsed().unwrap()`
- **Risk:** Panics if system time goes backwards
- **Proposed fix:** Use `unwrap_or_default()`
- **Commit:** `fix(helpers): handle system time edge case`

### 14. Error unwrap after is_some check
- [ ] Fix
- **File:** `src/shd/maker/exec/mod.rs:171,194,240`
- **Code:** `error.clone().unwrap()` after checking `is_some()`
- **Risk:** Fragile pattern, could break if logic changes
- **Proposed fix:** Use `if let Some(err)` pattern
- **Commit:** `refactor(exec): use if-let instead of is_some + unwrap`

---

## LOW (acceptable / unlikely to fail)

### 15. serde_json::to_value unwraps
- [ ] Fix (probably skip)
- **File:** `src/shd/data/pub.rs:45,55,65,75`, `src/shd/types/config.rs:195`
- **Code:** `serde_json::to_value(...).unwrap()`
- **Risk:** Very low - serializing known types
- **Proposed fix:** Could use `expect()` with message if desired
- **Commit:** N/A

### 16. NetworkName::from_str unwrap
- [ ] Fix (probably skip)
- **File:** `src/shd/types/config.rs:315,322`
- **Code:** `NetworkName::from_str(&self.network_name).unwrap()`
- **Risk:** Low if network_name validated at config load
- **Proposed fix:** Validate at config load time
- **Commit:** N/A
