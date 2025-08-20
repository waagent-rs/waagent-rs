#
# spec file for package waagent-rs
#

Name:           waagent-rs
Version:        1.0.2
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
%license LICENSE-APACHE
%license LICENSE-MIT
%doc README.md



%changelog
* Wed Auf 20 2025 Francisco Ortiz (francisco.ortiz@microsoft.com) - 1.0.2
- Making changes o files, RUL, sources and build
* Tue Aug 19 2025 Francisco Ortiz (francisco.ortiz@microsoft.com) - 1.0.1
- Initial package
