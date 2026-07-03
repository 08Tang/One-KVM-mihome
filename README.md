<div align="center">
  <h1>One-KVM-mihome</h1>
  <p><strong>米家智能家居控制面板 + One-KVM</strong></p>

  <p><a href="README.md">简体中文</a> · <a href="README.en.md">English</a></p>

  [![GitHub stars](https://img.shields.io/github/stars/08Tang/One-KVM-mihome?style=social)](https://github.com/08Tang/One-KVM-mihome/stargazers)
  [![GitHub forks](https://img.shields.io/github/forks/08Tang/One-KVM-mihome?style=social)](https://github.com/08Tang/One-KVM-mihome/network/members)
  [![GitHub issues](https://img.shields.io/github/issues/08Tang/One-KVM-mihome)](https://github.com/08Tang/One-KVM-mihome/issues)
</div>

---

## ⚠️ 重要声明

本项目 **非 One-KVM 官方项目**，是基于 [One-KVM](https://github.com/mofeng-git/One-KVM) 的二次创作项目。

**原版 One-KVM 地址**: https://github.com/mofeng-git/One-KVM

**感谢**：感谢 [One-KVM](https://github.com/mofeng-git/One-KVM) 项目及其贡献者提供的优秀基础框架。

---

## 📖 项目概述

**One-KVM-mihome** 在保留 [One-KVM](https://github.com/mofeng-git/One-KVM) IP-KVM 核心功能的基础上，新增了米家智能家居控制面板功能。

- **米家智能家居控制面板**: 基于 Flask 的 Web 控制面板，支持多设备管理与控制
- **One-KVM**: Rust 编写的开放轻量 IP-KVM 解决方案，实现 BIOS 级远程管理

---

## ✨ 米家智能家居控制面板

### 功能特性

| 功能 | 说明 |
|------|------|
| Web 控制面板 | 浏览器访问，响应式布局，支持移动端 |
| 设备管理 | 通过命令行工具添加、编辑、删除米家设备 |
| 实时状态 | 查看设备在线状态、运行工况、传感器数据 |
| 远程控制 | 开关控制、重启操作等 |
| 安全认证 | 密码登录，会话管理，支持修改密码 |

### 支持设备

| 设备型号 | 名称 | 功能 |
|---------|------|------|
| `cuco.plug.v3` | 米家智能插座3 | 开关、功率监测、温度、指示灯 |
| `cddz.plug.pc01w` | 开机卡 | 开关机、软重启、强制重启、运行工况 |

---

## 📁 项目结构

```
One-KVM-mihome/
├── One-KVM/                    # One-KVM 源码（原版项目）
│   └── ...
└── MiHome/                     # 米家控制面板
    ├── dist/                   # ✅ 可执行程序目录（直接运行）
    │   ├── mihome              # Web 服务入口（当前架构）
    │   ├── set                 # 设备管理命令行工具（当前架构）
    │   ├── install             # 服务安装工具（当前架构）
    │   ├── auth.json           # 米家账号认证信息
    │   ├── config.json         # 应用配置
    │   ├── devices.json        # 设备列表配置
    │   └── x86/                # x86_64 架构可执行程序
    │       ├── mihome          # Web 服务入口（x86）
    │       ├── set             # 设备管理命令行工具（x86）
    │       ├── install         # 服务安装工具（x86）
    │       └── config.json     # 应用配置（x86）
    ├── mihome.py               # Web 服务源码
    ├── set.py                  # 设备管理源码
    ├── install.py              # 服务安装源码
    ├── build.sh                # 编译脚本（从源码编译）
    ├── app/                    # Flask 应用源码
    └── mijia-api/              # 米家 API 库源码
```

---

## ⚡ 快速开始

### 米家控制面板

**运行 dist 目录中的可执行程序（推荐）**：

```bash
cd MiHome/dist

# 1. 登录米家账号
./set login

# 2. 添加设备
./set add

# 3. 启动 Web 服务
./mihome
```

访问 `http://<IP>:7123/webui`，默认密码：`123456`

**服务管理**：

```bash
cd MiHome/dist
sudo ./install
```

### 从源码编译（如需修改源码）

```bash
cd MiHome
bash build.sh
```

编译产物会生成到 `MiHome/dist/` 目录下。

### One-KVM 主程序

请访问 [One-KVM 官方仓库](https://github.com/mofeng-git/One-KVM) 获取安装和使用说明。

---

## 🔧 命令行工具 (set)

```bash
./set <命令>
```

| 命令 | 说明 |
|------|------|
| `login` | 登录米家账号 |
| `list` | 查看已配置设备 |
| `add` | 添加新设备 |
| `edit` | 编辑设备信息 |
| `delete` | 删除设备 |
| `query` | 查询设备状态 |
| `types` | 查看支持的设备类型 |

---

## 🔗 相关链接

- **One-KVM 官方**: https://github.com/mofeng-git/One-KVM
- **One-KVM 文档**: https://docs.one-kvm.cn/