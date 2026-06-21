# skillsync

多 Git 仓库状态同步与更新工具。批量管理本地 Git 仓库——检查落后状态、并发拉取更新、递归扫描添加仓库，并支持将 SSH 私钥安全存储到系统钥匙串。

## 子命令

| 命令 | 说明 |
|---|---|
| `status` | 检查所有仓库的落后/领先状态 |
| `update` | 并发拉取所有落后仓库的更新 |
| `add <路径...>` | 添加一个或多个仓库到监控列表 |
| `add-recursive [路径]` | 递归扫描目录，添加其下所有 Git 仓库 |
| `remove <路径...>` | 从监控列表移除仓库 |
| `remove-recursive [路径]` | 递归扫描目录，移除其下所有 Git 仓库 |
| `clone <仓库地址...>` | 克隆仓库并自动加入监控列表 |
| `concurrency` | 查看当前并发数 |
| `set-concurrency <N>` | 设置并发数（默认 20） |
| `key import <路径>` | 导入 SSH 私钥到系统钥匙串 |
| `key remove` | 从钥匙串删除私钥 |
| `key status` | 查看私钥存储状态 |

## 安装

```sh
make install
```

默认安装到 `/usr/local`，可通过变量覆盖：

```sh
make install PREFIX=$HOME/.local
```

该命令会编译 release 版本、生成 man 页面，然后将二进制和 man 页面复制到目标目录。卸载：

```sh
make uninstall
```

## 生成 man 页面

```sh
make man
```

生成的 man 文件位于 `target/man/` 目录。需要 release 模式编译，`make install` 会自动执行此步骤。