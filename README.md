# waagent-rs

Azure Agent, in Rust

Rewriting a critical piece of software that runs on all Azure VMs, in a lean implementation of a memory safe lenguage.

The current status of the project, is a pre-packaged software that you can run on both Windows and Linux to have a secure and lightweight experience in testing and development environments.

Note: We don't recommend using ```waagent-rs``` in production, *yet*. Please allow some time for the project to reach maturity. 

## Main differences

This are some of the main differences with (python) WALinuxAgent:
- Built entirely in rust
- Does not run as root, but as user waagent-rs
- Works on Linux and Windows
- Packaged for multiple Linux distributions and versions, as well as MSI for Windows

## How to install

Look for one of the pre-packaged in the release page and install.

|Distributions|Version|Archs available|Download|
|-------------|-------|---------------|--------|
|Azure Linux| 3.0| x86_64 and aarch64|[x86_64](https://github.com/waagent-rs/waagent-rs/releases/download/v0.1.1/waagent-rs-0.1.1-1.azl3.x86_64.rpm) [aarch64 rpm](https://github.com/waagent-rs/waagent-rs/releases/download/v0.1.1/waagent-rs-0.1.1-1.azl3.aarch64.rpm) |
|RHEL, Oracle Linux, Alma Linux, etc|8 and 9|x86_64 and aarch64|[EL8 x86_64 rpm](https://github.com/waagent-rs/waagent-rs/releases/download/v0.1.1/waagent-rs-0.1.1-1.el8.x86_64.rpm) [EL8 aarch64 rpm](https://github.com/waagent-rs/waagent-rs/releases/download/v0.1.1/waagent-rs-0.1.1-1.el8.aarch64.rpm) [EL9 x86_64 rpm](https://github.com/waagent-rs/waagent-rs/releases/download/v0.1.1/waagent-rs-0.1.1-1.el9.x86_64.rpm) [EL9 aarch64](https://github.com/waagent-rs/waagent-rs/releases/download/v0.1.1/waagent-rs-0.1.1-1.el9.aarch64.rpm) [EL10 x86_64](https://github.com/waagent-rs/waagent-rs/releases/download/v0.1.1/waagent-rs-0.0.136-1.el10.x86_64.rpm)  |
|Suse|15|x86_64 and aarch64|[x86_64](https://github.com/waagent-ts/waagent-rs/releases/download/v0.1.1/waagent-rs-0.1.1-1.x86_64.rpm) [aarch64 rpm](https://github.com/waagent-rs/waagent-rs/releases/download/v0.1.1/waagent-rs-0.1.1-1.aarch64.rpm)|
|Ubuntu and Debian|all LTS under support|x86_64 and aarch64|[amd64 deb](https://github.com/waagent-rs/waagent-rs/releases/download/v0.1.1-1/waagent-rs-poc_0.1.1-1_amd64.deb) [arm64 deb](https://github.com/waagent-rs/waagent-rs/releases/download/v0.1.1-1/waagent-rs-poc_0.1.1-1_arm64.deb) |
|Windows|Server 2025 and others|x86_64|[x86_64 MSI](https://github.com/waagent-rs/waagent-rs/releases/download/v0.1.1-1/waagent-rs-poc-0.1.1-x86_64.msi) |

In Linux, will create the waagent-rs service.

```
sudo systemctl stop walinuxagent
sudo systemctl start waagent-rs
```

Wait a few seconds, and go and check the Agent Status of the VM in the portal.


## For Developers

### How to build

```
git clone https://github.com/waagent-rs/waagent-rs.git
cd waagent-rs
cargo build --release
```

## Future work
- Improve documentation for customers and developers
- Add [Azure init](https://github.com/Azure/azure-init) for provisioning
- Improve logging for Windows
- Refine the rules of sudoers.d so that it only runs commands like iptables or the install of extensions
- Add more code coverage (testing)
