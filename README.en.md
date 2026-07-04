<div align="center">
  <h1>One-KVM-mihome</h1>
  <p><strong>MiHome Smart Home Control Panel + One-KVM</strong></p>

  <p><a href="README.md">简体中文</a> · <a href="README.en.md">English</a></p>

  [![GitHub stars](https://img.shields.io/github/stars/08Tang/One-KVM-mihome?style=social)](https://github.com/08Tang/One-KVM-mihome/stargazers)
  [![GitHub forks](https://img.shields.io/github/forks/08Tang/One-KVM-mihome?style=social)](https://github.com/08Tang/One-KVM-mihome/network/members)
  [![GitHub issues](https://img.shields.io/github/issues/08Tang/One-KVM-mihome)](https://github.com/08Tang/One-KVM-mihome/issues)
</div>

---

## ⚠️ Important Notice

This project is **NOT the official One-KVM project**, it is a fork based on [One-KVM](https://github.com/mofeng-git/One-KVM).

**Official One-KVM**: https://github.com/mofeng-git/One-KVM

**Thanks**: Special thanks to the [One-KVM](https://github.com/mofeng-git/One-KVM) project and its contributors for providing an excellent foundation.

---

## 📖 Overview

**One-KVM-mihome** adds MiHome (Xiaomi Smart Home) control panel functionality while preserving the core IP-KVM features of [One-KVM](https://github.com/mofeng-git/One-KVM).

- **MiHome Control Panel**: Flask-based web control panel for multi-device management
- **One-KVM**: Rust-based open, lightweight IP-KVM solution for BIOS-level remote management

---

## ✨ MiHome Smart Home Control Panel

### Features

| Feature | Description |
|---------|-------------|
| Web Control Panel | Browser-based access, responsive layout, mobile support |
| Device Management | CLI tool for adding, editing, and deleting MiHome devices |
| Real-time Status | View device online status, operating conditions, sensor data |
| Remote Control | Switch control, reboot operations, etc. |
| Security | Password login, session management, password change support |

### Supported Devices

| Model | Name | Features |
|-------|------|----------|
| `cuco.plug.v3` | Mi Smart Plug 3 | Switch, power monitoring, temperature, indicator |
| `cddz.plug.pc01w` | Boot Card | Power on/off, soft reboot, force reboot, status |

---

## 📁 Project Structure

```
One-KVM-mihome/
├── One-KVM/                    # One-KVM source code (original project)
│   └── ...
└── MiHome/                     # MiHome control panel
    ├── dist/                   # ✅ Executable programs (run directly)
    │   ├── mihome              # Web service entry (current arch)
    │   ├── set                 # Device management CLI (current arch)
    │   ├── install             # Service installation tool (current arch)
    │   ├── auth.json           # MiHome account auth info
    │   ├── config.json         # Application config
    │   ├── devices.json        # Device list config
    │   └── x86/                # x86_64 architecture binaries
    │       ├── mihome          # Web service entry (x86)
    │       ├── set             # Device management CLI (x86)
    │       ├── install         # Service installation tool (x86)
    │       └── config.json     # Application config (x86)
    ├── mihome.py               # Web service source
    ├── set.py                  # Device management source
    ├── install.py              # Service installation source
    ├── build.sh                # Build script (compile from source)
    ├── app/                    # Flask application source
    └── mijia-api/              # MiHome API library source
```

---

## ⚡ Quick Start

### MiHome Control Panel

**Run executable programs from dist directory (Recommended)**:

```bash
cd MiHome/dist

# 1. Login to MiHome account
./set login

# 2. Add devices
./set add

# 3. Start web service
./mihome
```

Access `http://<IP>:7123/webui`, default password: `123456`

**Service Management**:

```bash
cd MiHome/dist
sudo ./install
```

### Build from Source (if modifications needed)

```bash
cd MiHome
bash build.sh
```

Build artifacts will be generated in `MiHome/dist/`.

### One-KVM Core

Please visit the [Official One-KVM Repository](https://github.com/mofeng-git/One-KVM) for installation and usage instructions.

---

## 🔧 CLI Tool (set)

```bash
./set <command>
```

| Command | Description |
|---------|-------------|
| `login` | Login to MiHome account |
| `list` | View configured devices |
| `add` | Add new device |
| `edit` | Edit device information |
| `delete` | Delete device |
| `query` | Query device status |
| `types` | View supported device types |

---

## 🔗 Related Links

- **Official One-KVM**: https://github.com/mofeng-git/One-KVM
- **One-KVM Documentation**: https://docs.one-kvm.cn/