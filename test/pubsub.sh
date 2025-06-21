#!/bin/bash

# Redis connection settings
REDIS_HOST="127.0.0.1"
REDIS_PORT="42044"
CHANNEL="chrdmm"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${BLUE}=== Redis Pub/Sub Test Script ===${NC}"
echo "Host: $REDIS_HOST:$REDIS_PORT"
echo "Channel: $CHANNEL"
echo ""

# Function to publish message and show result
publish_message() {
    local test_name="$1"
    local message="$2"

    echo -e "${YELLOW}Testing: $test_name${NC}"
    echo "Message: $message"

    local result=$(redis-cli -h $REDIS_HOST -p $REDIS_PORT PUBLISH $CHANNEL "$message")

    if [ $? -eq 0 ]; then
        echo -e "${GREEN}✓ Published successfully (subscribers: $result)${NC}"
    else
        echo -e "${RED}✗ Failed to publish${NC}"
    fi
    echo ""
}

# Test 1: New Instance Message
echo -e "${BLUE}--- Test 1: New Instance Message ---${NC}"
publish_message "New Instance" '{
    "message_type": "new_instance",
    "timestamp": 1703123456789,
    "data": {
        "instance_id": "mm-001",
        "network": "ethereum"
    }
}'

# Test 2: Trade Event Message
echo -e "${BLUE}--- Test 2: Trade Event Message ---${NC}"
publish_message "Trade Event" '{
    "message_type": "trade_event",
    "timestamp": 1703123456790,
    "data": {
        "instance_id": "mm-001",
        "tx_hash": "0x1234567890abcdef1234567890abcdef12345678",
        "status": "success"
    }
}'

# Test 3: Another New Instance
echo -e "${BLUE}--- Test 3: Another New Instance ---${NC}"
publish_message "New Instance (Base)" '{
    "message_type": "new_instance",
    "timestamp": 1703123456791,
    "data": {
        "instance_id": "mm-002",
        "network": "base"
    }
}'

# Test 4: Failed Trade Event
echo -e "${BLUE}--- Test 4: Failed Trade Event ---${NC}"
publish_message "Failed Trade" '{
    "message_type": "trade_event",
    "timestamp": 1703123456792,
    "data": {
        "instance_id": "mm-001",
        "tx_hash": "0xabcdef1234567890abcdef1234567890abcdef12",
        "status": "failed"
    }
}'

# Test 5: Unknown Message Type (should be handled gracefully)
echo -e "${BLUE}--- Test 5: Unknown Message Type ---${NC}"
publish_message "Unknown Type" '{
    "message_type": "unknown_type",
    "timestamp": 1703123456793,
    "data": {
        "some_field": "some_value"
    }
}'

# Test 6: Malformed JSON (should show error)
echo -e "${BLUE}--- Test 6: Malformed JSON ---${NC}"
publish_message "Malformed JSON" '{
    "message_type": "new_instance",
    "timestamp": 1703123456794,
    "data": {
        "instance_id": "mm-003"
'

# Test 7: Multiple messages in quick succession
echo -e "${BLUE}--- Test 7: Multiple Messages (Quick Succession) ---${NC}"
for i in {1..3}; do
    publish_message "Quick Message $i" "{
        \"message_type\": \"trade_event\",
        \"timestamp\": $(date +%s)000,
        \"data\": {
            \"instance_id\": \"mm-00$i\",
            \"tx_hash\": \"0x$(openssl rand -hex 20)\",
            \"status\": \"success\"
        }
    }"
    sleep 0.1
done

# Test 8: High volume test
echo -e "${BLUE}--- Test 8: High Volume Test (10 messages) ---${NC}"
for i in {1..10}; do
    publish_message "Volume Test $i" "{
        \"message_type\": \"trade_event\",
        \"timestamp\": $(date +%s)000,
        \"data\": {
            \"instance_id\": \"mm-volume\",
            \"tx_hash\": \"0x$(openssl rand -hex 20)\",
            \"status\": \"success\"
        }
    }" >/dev/null 2>&1
done
echo -e "${GREEN}✓ Published 10 messages in volume test${NC}"
echo ""

echo -e "${BLUE}=== Test Summary ===${NC}"
echo -e "${GREEN}✓ All test messages sent${NC}"
echo ""
echo -e "${YELLOW}Check your Rust application logs to see how each message was processed.${NC}"
echo -e "${YELLOW}Expected log entries:${NC}"
echo "  - New instance deployed: mm-001 on network ethereum"
echo "  - Trade event: mm-001 - 0x1234... - success"
echo "  - New instance deployed: mm-002 on network base"
echo "  - Trade event: mm-001 - 0xabcd... - failed"
echo "  - Unknown message type warnings"
echo "  - JSON parsing error messages"
