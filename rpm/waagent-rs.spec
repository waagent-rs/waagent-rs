#
# spec file for package sudo-rs
#

Name:           waagent-rs
Version:        1.0.1
Release:        1%{?dist}
Summary:        Azure Agent, in Rust

License:        MIT OR Apache-2.0
URL:            
Source0:        

BuildRequires:  rust
BuildRequires:  cargo
Buildrequires:  gcc
BuildRequires:  openssl-dev

%global debug_package %{nil}

%description
Azure Agent, in Rust

%prep
%setup -n waagent-rs-%{version}


%build
cargo build --release --features 

%install
rm -rf $RPM_BUILD_ROOT
%make_install


%files
%license LICENSE-APACHE
%license LICENSE-MIT
%doc README.md



%changelog
* Tue Aug 19 2025 Francisco Ortiz (francisco.ortiz@microsoft.com) - 1.0.1
- Initial package
