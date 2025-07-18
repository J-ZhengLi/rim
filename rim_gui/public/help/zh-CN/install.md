# 首次安装

首次启动程序时，它将进入初始化模式，引导您在环境中安装 Rust。

## 1. 开始安装

在主页点击 <button class="primary-button">安装</button> 按钮开始安装.

或者，如果您已有工具套件清单，可点击页面底部的 <u>使用套件清单安装</u>。此操作将会覆盖内置的默认工具套件清单。

## 2. 安装配置

在此步骤中，您可以配置 Rust 安装选项。
首先设置一个安装路径，这是 RIM 存储大部分文件（包括自身二进制文件）的位置。

您将看到一个输入栏，右侧有 <button class="primary-button">选择目录</button> 按钮，可在此输入安装路径或使用文件选择器选取：

<img src="/help/zh-CN/images/file-picker.png" width="100%"/>

您还可点击 `高级选项 ▼` 标签展开包含高级配置选项的完整区域。
将鼠标悬停在任意选项上可查看提示，若不了解其作用，不建议修改这些选项。

> 部分选项在 `下载源配置` 下可能被禁用，这表示它们由工具套件清单强制设定，属于正常现象。

## 3. 组件定制

### 3.1 配置方案选择

组件分为两种配置方案：`精简版`、`标准版`。点击选择最适合的方案，或点击 `自定义` 进行高级组件选择。

### 3.2 组件选择

若在配置文件页面选择 `自定义`，将进入组件选择页面：

<img src="/help/zh-CN/images/component-select.png" width="100%"/>

点击左侧复选框勾选要安装的组件，点击组件名称可在右侧面板查看详细描述。

### 3.3 包源覆盖

部分组件（如 `VS Build Tools`）允许覆盖其包源。

此时您可以：

1. 使用默认（官方）源继续
2. 提供自定义包（支持本地文件路径或网页 URL）

<img src="/help/zh-CN/images/vsbt-src-ovr.png" width="100%"/>

## 4. 确认信息

开始安装前，RIM 将最后一次请您确认安装配置。

仔细检查安装路径和待安装组件，然后点击 <button class="primary-button">安装</button> 按钮开始安装。

## 5. 安装进度

安装期间将显示整体进度的进度条，您也可点击 `显示详情 ▼` 查看详细输出。

<img src="/help/zh-CN/images/start-install.png" width="100%"/>

## 6. 完成安装

这是初始化模式的最后一步。程序自动关闭前，将询问您是否希望：

1. 完成后打开
    
    退出初始化模式后以管理模式重新打开。
2. 创建桌面快捷方式

    为 RIM 创建桌面快捷方式。
    > 在 Linux 平台，同时会创建应用菜单快捷方式。

勾选所需选项后，点击 <button class="primary-button">完成</button> 按钮退出程序。

<img src="/help/zh-CN/images/install-finish.png" width="100%"/>

<style>
.primary-button {
    color: white;
    background-color: #5b98d8;
    margin-inline: 5px;
    border: none;
    border-radius: 20px;
    box-shadow: 0 0 0 2px rgba(255, 255, 255, .4);
    font-weight: bold;
    white-space: nowrap;
    overflow: hidden;
    padding: 3px 10px;
}
</style>
