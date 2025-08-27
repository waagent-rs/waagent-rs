#
# spec file for package waagent-rs
#

Name:           waagent-rs
Version:        0.1.0
Release:        1%{?dist}
Summary:        Azure Agent, in Rust

License:        MIT OR Apache-2.0
URL:            https://github.com/waagent-rs/waagent-rs.git        
Source:         %{url}/archive/v%{version}/waagent-rs-%{version}.tar.gz

BuildRequires:  rust
BuildRequires:  cargo
BuildRequires:  gcc
BuildRequires:  openssl-dev


%description
Azure Agent, in Rust

%prep
%setup -n waagent-rs-%{version}


%build
cargo build --release

%install
rm -rf $RPM_BUILD_ROOT
%make_install


%files
/usr/bin/waagent-rc
/usr/lib/systemd/system/waagent-rc
%license LICENSE-APACHE
%doc README.md



%changelog
* Tue Aug 19 2025 Francisco Ortiz (francisco.ortiz@microsoft.com) - 0.1.0
- Initial package
