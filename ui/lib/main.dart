// main.dart — ThemeSwitcher2 玻璃仪表盘入口
// 窗口: flutter_acrylic(Win10 亚克力)+ window_manager(隐藏标题栏/窗口按钮)
// 玻璃: liquid_glass_widgets(按官方 SKILL 规则: 玻璃只用于导航层, 内容卡 solid)
import 'package:flutter/material.dart';
import 'package:flutter_acrylic/flutter_acrylic.dart';
import 'package:liquid_glass_widgets/liquid_glass_widgets.dart';
import 'package:window_manager/window_manager.dart';

import 'dashboard.dart';

Future<void> main() async {
  WidgetsFlutterBinding.ensureInitialized();

  // 玻璃组件库: 预热 shader, 防首帧白闪(官方要求)
  await LiquidGlassWidgets.initialize();

  await windowManager.ensureInitialized();
  const opts = WindowOptions(
    size: Size(1180, 880),
    minimumSize: Size(1000, 760),
    title: 'ThemeSwitcher2',
    titleBarStyle: TitleBarStyle.hidden,
    backgroundColor: Colors.transparent,
  );
  await windowManager.waitUntilReadyToShow(opts, () async {
    // Win10: 窗口级亚克力, 桌面透过玻璃
    await Window.setEffect(effect: WindowEffect.acrylic, color: const Color(0xCC0E1420));
    await windowManager.show();
    await windowManager.focus();
  });

  runApp(LiquidGlassWidgets.wrap(
    brightnessResolver: Theme.maybeBrightnessOf, // 让玻璃组件跟随 MaterialApp 的 ThemeMode
    theme: const GlassThemeData(),               // iOS 27 光学(默认档)
    child: const ThemeSwitcher2App(),
  ));
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
