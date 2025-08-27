#
# spec file for package waagent-rs
#

Name:           waagent-rs
Version:        0.1.0
Release:        1%{?dist}
Summary:        Azure Agent, in Rust

License:        MIT OR Apache-2.0
URL:            https://github.com/waagent-rs/waagent-rs
Source:         %{url}/archive/v%{version}/waagent-rs-%{version}.tar.gz

BuildRequires:  gcc
BuildRequires:  openssl-devel
BuildRequires:  curl

%description
Azure Agent, in Rust

%prep
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
source $HOME/.cargo/env
rustup install 1.88.0
rustup default 1.88.0

%setup -n waagent-rs-%{version}


%build
source $HOME/.cargo/env
cargo build --release

%install
install -Dm0755 target/release/visudo %{buildroot}/usr/lib/cargo/bin/visudo
install -Dm0755 init/systemd/waagent-rs.service %{buildroot}/usr/lib/systemd/system/waagent-rs.service

%pre
getent passwd waagent-rs >/dev/null || useradd -r -d /nonexistent -s /usr/sbin/nologin waagent-rs

%files
/usr/bin/waagent-rs-poc
/usr/lib/systemd/system/waagent-rs.service
%license LICENSE
%doc README.md

%changelog
* Tue Aug 19 2025 Francisco Ortiz (francisco.ortiz@microsoft.com) - 0.1.0
- Initial package
