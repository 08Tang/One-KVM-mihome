<div align="center">
  <h1>One-KVM-mihome</h1>
  <p><strong>米家智能家居控制面板 + One-KVM</strong></p>

  <p><a href="README.md">简体中文</a> · <a href="README.en.md">English</a></p>

[![GitHub stars](https://img.shields.io/github/stars/08Tang/One-KVM-mihome?style=social)](https://github.com/08Tang/One-KVM-mihome/stargazers)
[![GitHub forks](https://img.shields.io/github/forks/08Tang/One-KVM-mihome?style=social)](https://github.com/08Tang/One-KVM-mihome/network/members)
[![GitHub issues](https://img.shields.io/github/issues/08Tang/One-KVM-mihome)](https://github.com/08Tang/One-KVM-mihome/issues)

</div>

***

## ⚠️ 重要声明

本项目 **非 One-KVM 官方项目**，是基于 [One-KVM](https://github.com/mofeng-git/One-KVM) 的二次创作项目。

**原版 One-KVM 地址**: <https://github.com/mofeng-git/One-KVM>

**感谢**：感谢 [One-KVM](https://github.com/mofeng-git/One-KVM) 项目及其贡献者提供的优秀基础框架。

***

## 📖 项目概述

**One-KVM-mihome** 在保留 [One-KVM](https://github.com/mofeng-git/One-KVM) IP-KVM 核心功能的基础上，新增了米家智能家居控制面板功能。

- **米家智能家居控制面板**: Rust 编写的米家智能家居控制面板，实现智能设备远程控制
- **One-KVM**: Rust 编写的开放轻量 IP-KVM 解决方案，实现 BIOS 级远程管理

***

## ✨ 米家智能家居控制面板

### 功能特性

| 功能         | 说明                                          |
| ---------- | ------------------------------------------- |
| Web 控制面板   | 基于浏览器的智能家居管理界面，支持密码登录                       |
| 设备控制       | 远程控制米家智能设备，支持开关、参数设置                        |
| API 接口     | 提供 RESTful API，支持 SHA256 签名验证               |
| 多架构支持      | Linux (x86\_64/arm64/armv7) + Windows (x64) |
| systemd 服务 | Linux 下支持开机自启                               |
| 配置工具       | 独立的 `mihome-set` 命令行工具，交互式配置                |

### 支持设备

| 设备      | 型号                | 支持功能                                         |
| ------- | ----------------- | -------------------------------------------- |
| 米家智能插座3 | `cuco.plug.v3`    | 开关、实时功率(W)、设备温度(°)、耗电量(度)、故障状态、上电默认状态、指示灯、童锁 |
| 开机卡     | `cddz.plug.pc01w` | 通电开机/断电关机、运行状况(9种状态)、上电默认状态、指示灯、电源开关、重启      |

***

## ⚡ 快速开始

### 米家控制面板

构建产物见 [MiHome-Rust Releases](https://github.com/08Tang/MiHome-Rust/releases)。以下为常见安装方式的简要步骤。

#### 使用 deb 安装（Debian / Ubuntu）

从 Releases 下载与本机架构匹配的 `mihome_*.deb`，在包所在目录执行：

```bash
sudo apt update
sudo apt install ./mihome_0.x.x_<arch>.deb
```

将文件名中的版本号与架构替换为实际下载的包名。安装后 systemd 服务会自动启用并开机自启。

**系统要求：**

| 架构              | deb 包名               | 最低系统要求                     |
| --------------- | -------------------- | -------------------------- |
| x86\_64 (amd64) | `mihome_*_amd64.deb` | Debian 11+ / Ubuntu 20.04+ |
| ARM64 (arm64)   | `mihome_*_arm64.deb` | Debian 11+ / Ubuntu 20.04+ |
| ARMv7 (armhf)   | `mihome_*_armhf.deb` | Debian 11+ / Ubuntu 20.04+ |

**依赖：** libc6 >= 2.31，libssl1.1 >= 1.1.0 或 libssl3 >= 3.0.0

#### Windows

从 Releases 下载 `mihome-windows-x64.zip`，解压后直接运行。

**系统要求：** Windows 10 (1809+) / Windows 11 / Windows Server 2019+

Windows 版本为独立可执行文件，无需额外依赖。

#### 配置（使用 mihome-set）

```bash
# 第一步：登录米家账号
mihome-set login

# 第二步：添加设备
mihome-set add

# 第三步：设置 API 密钥（建议随机）
mihome-set api_key

# 第四步（可选）：关闭 WebUI
mihome-set webui off
```

Windows 用户使用 `.\mihome-set.exe` 替代 `mihome-set`。

> 仅搭配 One-KVM-mihome 使用时建议关闭 Web 面板：`mihome-set webui off`

### One-KVM 主程序

下载：[releases](https://github.com/08Tang/One-KVM-mihome/releases)  
请访问 [One-KVM 官方仓库](https://github.com/mofeng-git/One-KVM) 获取使用说明。

***

## 🔧 命令行工具 (mihome-set)

`mihome-set` 支持两种模式：**命令行模式**（带参数）和**交互模式**（不带参数）。

| 命令         | 说明                       | 示例                                             |
| ---------- | ------------------------ | ---------------------------------------------- |
| `login`    | 登录米家账号，保存认证信息到 auth.json | `mihome-set login`                             |
| `list`     | 查看已配置的设备列表               | `mihome-set list`                              |
| `add`      | 从米家账号获取设备列表，选择添加到控制列表    | `mihome-set add`                               |
| `edit`     | 编辑已配置设备的名称               | `mihome-set edit`                              |
| `delete`   | 删除已配置的设备                 | `mihome-set delete`                            |
| `query`    | 查询设备实时状态（支持查询单个或全部）      | `mihome-set query`                             |
| `types`    | 查看支持的设备类型及功能             | `mihome-set types`                             |
| `password` | 修改 WebUI 登录密码（留空随机生成）    | `mihome-set password`                          |
| `api_key`  | 修改 API 密钥（留空随机生成 32 位）   | `mihome-set api_key`                           |
| `webui`    | 开启/关闭 WebUI              | `mihome-set webui on` 或 `mihome-set webui off` |
| `help`     | 显示帮助信息                   | `mihome-set help`                              |

不带任何参数运行 `mihome-set`，进入交互模式，可循环执行命令，输入 `exit` 退出。

***

## 🔗 相关链接

- **One-KVM 官方**: <https://github.com/mofeng-git/One-KVM>
- **One-KVM 文档**: <https://docs.one-kvm.cn/>

