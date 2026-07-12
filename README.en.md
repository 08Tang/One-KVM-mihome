<div align="center">
  <h1>One-KVM-mihome</h1>
  <p><strong>MiHome Smart Home Control Panel + One-KVM</strong></p>

  <p><a href="README.md">简体中文</a> · <a href="README.en.md">English</a></p>

  [![GitHub stars](https://img.shields.io/github/stars/08Tang/One-KVM-mihome?style=social)](https://github.com/08Tang/One-KVM-mihome/stargazers)
  [![GitHub forks](https://img.shields.io/github/forks/08Tang/One-KVM-mihome?style=social)](https://github.com/08Tang/One-KVM-mihome/network/members)
  [![GitHub issues](https://img.shields.io/github/issues/08Tang/One-KVM-mihome)](https://github.com/08Tang/One-KVM-mihome/issues)
</div>

***

## ⚠️ Important Notice

This project is **NOT the official One-KVM project**, it is a fork based on [One-KVM](https://github.com/mofeng-git/One-KVM).

**Official One-KVM**: <https://github.com/mofeng-git/One-KVM>

**Thanks**: Special thanks to the [One-KVM](https://github.com/mofeng-git/One-KVM) project and its contributors for providing an excellent foundation.

***

## 📖 Overview

**One-KVM-mihome** adds MiHome (Xiaomi Smart Home) control panel functionality while preserving the core IP-KVM features of [One-KVM](https://github.com/mofeng-git/One-KVM).

- **MiHome Control Panel**: A lightweight Xiaomi smart home control panel written in Rust for remote device management
- **One-KVM**: Rust-based open, lightweight IP-KVM solution for BIOS-level remote management

***

## ✨ MiHome Smart Home Control Panel

### Features

| Feature | Description |
|---------|-------------|
| Web Control Panel | Browser-based smart home management interface with password login |
| Device Control | Remote control of Mijia smart devices with switch and parameter settings |
| API Interface | RESTful API with SHA256 signature verification |
| Multi-architecture | Linux (x86_64/arm64/armv7) + Windows (x64) |
| systemd Service | Auto-start on boot for Linux |
| Configuration Tool | Standalone `mihome-set` CLI tool with interactive mode |

### Supported Devices

| Device | Model | Supported Functions |
|--------|-------|---------------------|
| Mijia Smart Plug 3 | `cuco.plug.v3` | Switch, real-time power (W), device temperature (°), energy consumption (kWh), fault status, default power-on state, indicator light, child lock |
| Boot Card | `cddz.plug.pc01w` | Power on/off, running status (9 states), default power-on state, indicator light, power switch, restart |

***

## ⚡ Quick Start

### MiHome Control Panel

Build artifacts are available on [MiHome-Rust Releases](https://github.com/08Tang/MiHome-Rust/releases). Below are brief installation steps for common methods.

#### Install via deb (Debian / Ubuntu)

Download the `mihome_*.deb` package matching your system architecture, then run in the package directory:

```bash
sudo apt update
sudo apt install ./mihome_0.x.x_<arch>.deb
```

Replace the version number and architecture with the actual downloaded package name. The systemd service will be automatically enabled and start on boot.

**System Requirements:**

| Architecture | deb Package | Minimum System Requirement |
|--------------|-------------|----------------------------|
| x86_64 (amd64) | `mihome_*_amd64.deb` | Debian 11+ / Ubuntu 20.04+ |
| ARM64 (arm64) | `mihome_*_arm64.deb` | Debian 11+ / Ubuntu 20.04+ |
| ARMv7 (armhf) | `mihome_*_armhf.deb` | Debian 11+ / Ubuntu 20.04+ |

**Dependencies:** libc6 >= 2.31, libssl1.1 >= 1.1.0 or libssl3 >= 3.0.0

#### Windows

Download `mihome-windows-x64.zip` from Releases, extract and run directly.

**System Requirements:** Windows 10 (1809+) / Windows 11 / Windows Server 2019+

The Windows version is a standalone executable with no additional dependencies.

#### Configuration (using mihome-set)

```bash
# Step 1: Log in to your Mijia account
mihome-set login

# Step 2: Add devices
mihome-set add

# Step 3: Set API key (random recommended)
mihome-set api_key

# Step 4 (optional): Disable WebUI
mihome-set webui off
```

Windows users: use `.\mihome-set.exe` instead of `mihome-set`.

#### Accessing the Web Control Panel

Open your browser and navigate to `http://<device IP>:7123/webui`. Default password: `123456` (change with `mihome-set password`).

> When using with One-KVM-mihome only, it is recommended to disable the web panel: `mihome-set webui off`

### One-KVM Core

Download: [releases](https://github.com/08Tang/One-KVM-mihome/releases)  
Please visit the [Official One-KVM Repository](https://github.com/mofeng-git/One-KVM) for usage instructions.

***

## 🔧 CLI Tool (mihome-set)

`mihome-set` supports two modes: **CLI mode** (with arguments) and **interactive mode** (without arguments).

| Command | Description | Example |
|---------|-------------|---------|
| `login` | Log in to Mijia account, save auth info to auth.json | `mihome-set login` |
| `list` | View configured device list | `mihome-set list` |
| `add` | Fetch device list from Mijia account and add to control list | `mihome-set add` |
| `edit` | Edit configured device name | `mihome-set edit` |
| `delete` | Delete a configured device | `mihome-set delete` |
| `query` | Query real-time device status (single or all) | `mihome-set query` |
| `types` | View supported device types and features | `mihome-set types` |
| `password` | Change WebUI login password (leave empty for random) | `mihome-set password` |
| `api_key` | Change API key (leave empty for random 32-char key) | `mihome-set api_key` |
| `webui` | Enable/disable WebUI | `mihome-set webui on` or `mihome-set webui off` |
| `help` | Display help information | `mihome-set help` |

Run `mihome-set` without any arguments to enter interactive mode. Type `exit` to quit.

***

## 🔗 Related Links

- **Official One-KVM**: <https://github.com/mofeng-git/One-KVM>
- **One-KVM Documentation**: <https://docs.one-kvm.cn/>