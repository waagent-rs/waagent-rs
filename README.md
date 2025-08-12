# waagent
Azure Agent, in Rust

## Main differences
This are some of the main differences with (python) WALinuxAgent:
- Built entirely in rust
- Does not run as root, but as user waagent-rs
- At least for now, it doesn't do provisioning. This function is preferred to leave it to cloud-init.

## How to build

```
git clone https://github.com/waagent-rs/waagent-rs.git
cd waagent-rs
cargo build --release
```

## How to test

With a VM in azure:

```
sudo useradd -u 999 -r -d /nonexistent -s /usr/sbin/nologin waagent-rs
sudo cp waagent-poc/sudoers.d/waagent-rs /etc/sudoers.d
sudo systemctl stop walinuxagent
sudo -u waagent-rs target/release/waagent-rs-poc

```

Wait a few seconds, and go and check the Agent Status of the VM in the portal.

## TODO
- Refine the rules of sudoers.d so that it only runs commands like iptables or the install of extensions
- Create issue for rust module "os_info" about Ubuntu 24.04.3 -> 24.4.0
