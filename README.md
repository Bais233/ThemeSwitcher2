# ThemeSwitcher2

按**日出日落**自动切换 Windows 深色/浅色模式与鼠标指针方案的托盘工具。Rust 重写版,原 C# 版见 [ThemeSwitcher](https://github.com/Bais233/ThemeSwitcher)。

- 托盘常驻,日出切浅色、日落切深色(各支持 ±720 分钟偏移)
- **启动补课**:错过事件才开机也会立刻对齐到当前时刻应有的模式
- **本地 NOAA 天文计算**,核心切换零网络依赖(联网仅用于定位)
- 深浅色各挂一个**鼠标指针方案**:支持注册表方案 + `C:\Windows\Cursors` 下未注册的社区指针包(自动识别角色)
- 指针包只写识别到的角色,未识别的保持现状,**绝不静默覆盖你的指针**
- 定位:IP 自动 / 坐标 / 地名(Nominatim)
- 单 exe,无运行时依赖;设置存 `%APPDATA%\ThemeSwitcher2\settings.ini`

## 使用

从 [Releases](https://github.com/Bais233/ThemeSwitcher2/releases) 下载(或 Actions artifact),两个文件:

- `ThemeSwitcher2.exe` — 引擎:启动托盘常驻,按 settings.ini 的 Location 定位
- `ThemeSwitcher2-UI.exe` — 可选的图形设置界面:与引擎放同一目录,托盘菜单"设置界面…"直接打开

托盘右键菜单:

- 当前状态(灰字,实时刷新)
- 切换到浅色模式 / 切换到深色模式
- 开机自启(勾选)
- 设置界面…(打开图形设置)
- 打开配置文件 / 重读配置
- 退出

## 配置(settings.ini)

| 键 | 默认 | 说明 |
|---|---|---|
| `SwitchApps` | `true` | 切换应用主题(`AppsUseLightTheme`) |
| `SwitchSystem` | `true` | 切换系统主题(任务栏/开始菜单) |
| `SunriseOffsetMin` | `0` | 日出偏移(正 = 推迟切浅色) |
| `SunsetOffsetMin` | `0` | 日落偏移(正 = 推迟切深色) |
| `SwitchCursor` | `false` | 切主题时同步切换指针方案 |
| `CursorLightScheme` | `(不修改)` | 浅色指针;`(不修改)` = 不碰指针 |
| `CursorDarkScheme` | `(不修改)` | 深色指针 |
| `Location` | 空 | `纬度,经度` / 地名 / 空 = IP 自动定位 |
| `Autostart` | `false` | 开机自启(HKCU Run) |

改完保存后托盘菜单选"重读配置"(或直接重启程序)生效。

## 云端编译

GitHub Actions 在每次 push 到 `main` 时用 `windows-latest` 做 release 编译,产物在 Actions artifact;打 `v*` tag 自动发 Release。

## License

MIT
