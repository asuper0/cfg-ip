<img src="./MadeWithSlint-logo-whitebg.png" align="right" width="140px">

# Cfg-IP

使用`Rust`编写的简单的Windows平台下IP配置工具。UI部分使用[Slint](https://github.com/slint-ui/slint)开发。

![config-ip-tool](https://github.com/asuper0/cfg-ip/assets/41113804/6f8b0a55-c187-44c2-af78-f270605f64f5)

## 使用说明

1. 当打开App时，系统当前的网络适配器会自动加载到中间的`Adapters`区域，你也可以点击`Refresh`按钮来手动刷新。
2. 选择一个网络适配器，在右侧区域可以看到详细信息。
3. 在步骤2. 中选择一项，然后点击`Load selected`按钮，适配器信息会加载到左侧区域。
4. 在左侧区域中按照你的需要进行修改，可以点击`Apply`按钮来使其生效，或点击`Save`按钮来保存，也可以点击`Load selected`按钮放弃更改。
   1. 当`dhcp on`未选中时，可以对一个适配器配置多个ip。`address list`列表中的每一行代表一个地址，ip地址和子网掩码必须是一一配对的，所以`address list`和`netmask`中的行数必须保持一致；
   2. `gateway` 和 `dns list` 同样支持配置多个。

如果你在步骤4. 中保存了一些配置，你可以在`Saved settings`区域中选择它们，并执行类似步骤2. 和步骤3. 的操作。

## 注意

本软件使用`netsh`命令行工具实现配置IP功能，我没有刻意隐藏控制台窗口，所以你在配置IP时会看到一个黑色的窗口出现，请在出现 "You can close the window now." 信息时手动关闭。
