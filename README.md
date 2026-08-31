Hello, im spazmatizm

im here to introduce my new creature, citadel package manager or how i call it CIT

Universal package manager for Linux that works with Arch, Debian, RPM, XBPS and APK repositories in one tool.

One manager - all distributions.

## Features

- Multi-format support: Arch (pacman), Debian (apt), RPM (dnf/yum), XBPS (xbps), APK (apk)
- Search packages across all configured repositories
- Install, remove and upgrade packages
- Update all packages with one command
- Source installation (sinstall)
- And it works on almost every distro because it does not need any PM to work


### how to download

curl -L curl -L https://github.com/spazmatizm-bit/cit/releases/download/alpha/cit -o cit

chmod +x cit

sudo mv cit /usr/local/bin/

## Usage

### Basic commands

Search for a package

cit search firefox
Install a package

cit install firefox
Remove a package

cit remove firefox
Upgrade a single package

cit upgrade firefox
Update all packages

cit update
List installed packages

cit list
Source installation

cit sinstall firefox
with specific repository

cit sinstall firefox --repo arch/core
Generate default configuration

cit generate-conf

## Configuration

Configuration file: `~/.cit.conf`


Supported repository types:
- `arch` - Arch Linux
- `deb` - Debian/Ubuntu
- `rpm` - Fedora/RHEL
- `xbps` - Void Linux
- `apk` - Alpine Linux

## Directory structure

Installed packages are stored in:

~/.citadel/

├── arch/core/

│ ├── firefox-120.0/

│ └── ...

├── debian/stable/

│ └── ...

└── ...

Binary files are automatically linked to `~/.local/bin/`

have fun using it, bye
