// main.dart — ThemeSwitcher2 玻璃仪表盘入口
// 玻璃: liquid_glass_widgets(按官方 SKILL 规则: 玻璃只用于导航层, 内容卡 solid)
// 窗口: flutter_acrylic(Win10 亚克力)+ window_manager(隐藏标题栏/窗口按钮)
// 日志: 启动各步骤落 %APPDATA%\ThemeSwitcher2\ui_log.txt(发布版排障用)
import 'dart:io';
import 'package:flutter/material.dart';
import 'package:flutter_acrylic/flutter_acrylic.dart';
import 'package:liquid_glass_widgets/liquid_glass_widgets.dart';
import 'package:window_manager/window_manager.dart';

import 'dashboard.dart';

void _log(String msg) {
  try {
    final appdata = Platform.environment['APPDATA'] ?? '.';
    final dir = Directory('$appdata\\ThemeSwitcher2');
    if (!dir.existsSync()) dir.createSync(recursive: true);
    File('${dir.path}\\ui_log.txt')
        .writeAsStringSync('${DateTime.now()}  $msg\n', mode: FileMode.append);
  } catch (_) {}
}

Future<void> main() async {
  WidgetsFlutterBinding.ensureInitialized();
  _log('startup: binding ok');

  try {
    await LiquidGlassWidgets.initialize();
    _log('liquid_glass initialize ok');
  } catch (e) {
    _log('liquid_glass initialize FAILED: $e');
  }

  await windowManager.ensureInitialized();
  _log('windowManager ready');

  const opts = WindowOptions(
    size: Size(1180, 880),
    minimumSize: Size(1000, 760),
    title: 'ThemeSwitcher2',
    titleBarStyle: TitleBarStyle.hidden,
    backgroundColor: Colors.transparent,
  );
  await windowManager.waitUntilReadyToShow(opts, () async {
    _log('window created (hidden)');
    // Win10: 窗口级亚克力。失败不阻断显示(玻璃降级为纯渐变)
    try {
      await Window.initialize();
      await Window.setEffect(effect: WindowEffect.acrylic, color: const Color(0xCC0E1420));
      _log('acrylic effect ok');
    } catch (e) {
      _log('acrylic effect FAILED: $e');
    }
    await windowManager.show(); // 无论玻璃成功与否都显示
    await windowManager.focus();
    _log('window shown');
  });

  _log('runApp');
  runApp(LiquidGlassWidgets.wrap(
    brightnessResolver: Theme.maybeBrightnessOf, // 玻璃组件跟随 MaterialApp 的 ThemeMode
    theme: const GlassThemeData(),               // iOS 27 光学(默认档)
    child: const ThemeSwitcher2App(),
  ));
  _log('runApp returned (window closed)');
}

class ThemeSwitcher2App extends StatelessWidget {
  const ThemeSwitcher2App({super.key});

  @override
  Widget build(BuildContext context) {
    return MaterialApp(
      title: 'ThemeSwitcher2',
      debugShowCheckedModeBanner: false,
      themeMode: ThemeMode.system,
      theme: ThemeData(
        brightness: Brightness.light,
        colorSchemeSeed: const Color(0xFF2563EB),
        useMaterial3: true,
      ),
      darkTheme: ThemeData(
        brightness: Brightness.dark,
        colorSchemeSeed: const Color(0xFF7FB2FF),
        useMaterial3: true,
      ),
      home: const DashboardScreen(),
    );
  }
}
