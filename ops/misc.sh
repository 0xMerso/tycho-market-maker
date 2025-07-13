# --- Utils ---

# docker network create tmm
# d1034ebdf0f2b17679a9beffbb6b60285dd92e25192085f9eb1fa628a824e1ad

# --- History ---

#     1  for pkg in docker.io docker-doc docker-compose docker-compose-v2 podman-docker containerd runc; do sudo apt-get remove $pkg; done
#     2  sudo apt update && sudo apt upgrade -y\nsudo apt install -y build-essential curl git ca-certificates gnupg lsb-release
#   141  vim .env.gh
#   142  echo $GH_MM
#   143  git clone https://0xMerso:$GH_MM@github.com/0xMerso/tycho-market-maker.git maker
#   155  sudo apt-get update\nsudo apt-get install -y ca-certificates curl gnupg lsb-release
#   156  sudo mkdir -p /etc/apt/keyrings\ncurl -fsSL https://download.docker.com/linux/ubuntu/gpg \\n  | sudo gpg --dearmor -o /etc/apt/keyrings/docker.gpg\necho \\n  "deb [arch=$(dpkg --print-architecture) signed-by=/etc/apt/keyrings/docker.gpg] \\n  https://download.docker.com/linux/ubuntu \\n  $(lsb_release -cs) stable" \\n  | sudo tee /etc/apt/sources.list.d/docker.list > /dev/null
#   157  sudo apt-get update\nsudo apt-get install -y docker-ce docker-ce-cli containerd.io docker-compose-plugin
#   158  sudo systemctl enable docker\nsudo systemctl start docker\nsudo systemctl status docker\ndocker compose version
#   160  docker ps

# --- --- --- Ubuntu VM --- --- ---
sudo apt-get update
sudo apt-get install pkg-config libssl-dev
