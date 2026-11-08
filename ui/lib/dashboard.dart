// dashboard.dart — 玻璃仪表盘(对应渲染稿 ts2_ui_render)
// 玻璃规则(官方 SKILL): 玻璃只铺导航层; 内容卡用 solid 档; GlassSwitch 允许在 solid 卡内
import 'dart:async';
import 'package:flutter/material.dart';
import 'package:liquid_glass_widgets/liquid_glass_widgets.dart';
import 'package:window_manager/window_manager.dart';

import 'api.dart';

class DashboardScreen extends StatefulWidget {
  const DashboardScreen({super.key});

  @override
  State<DashboardScreen> createState() => _DashboardScreenState();
}

class _DashboardScreenState extends State<DashboardScreen> {
  final api = EngineApi();
  Timer? timer;
  bool light = true;
  bool engineOk = true;
  String nextWhen = '';
  bool nextLight = false;
  String location = '';
  String sunrise = '';
  String sunset = '';
  List<String> schemes = [];
  Map<String, dynamic> cfg = {};
  bool cfgReady = false;
  String lastStatusKey = '';
  int navIndex = 0;

  @override
  void initState() {
    super.initState();
    _bootstrap();
    timer = Timer.periodic(const Duration(seconds: 1), (_) => _tick());
  }

  @override
  void dispose() {
    timer?.cancel();
    super.dispose();
  }

  Future<void> _bootstrap() async {
    try {
      final s = await api.schemes();
      final c = await api.settings();
      if (!mounted) return;
      setState(() {
        schemes = s;
        cfg = c;
        cfgReady = true;
      });
    } catch (_) {}
  }

  Future<void> _tick() async {
    try {
      final st = await api.status();
      final key = '${st['light']}|${st['nextWhen']}|${st['nextLight']}'
          '|${st['locationResolved']}|${st['sunrise']}|${st['sunset']}';
      if (key != lastStatusKey) {
        setState(() {
          engineOk = true;
          light = st['light'] == true;
          nextWhen = (st['nextWhen'] ?? '') as String;
          nextLight = st['nextLight'] == true;
          location = (st['locationResolved'] ?? '') as String;
          sunrise = (st['sunrise'] ?? '') as String;
          sunset = (st['sunset'] ?? '') as String;
          lastStatusKey = key;
        });
      } else if (!engineOk) {
        setState(() => engineOk = true);
      }
    } catch (_) {
      if (engineOk && mounted) setState(() => engineOk = false);
    }
  }

  // ---- 配置字段便捷访问 ----
  bool b(String k) => cfg[k] == true;
  int i(String k) => (cfg[k] as num?)?.toInt() ?? 0;
  String str(String k) => (cfg[k] ?? '') as String;

  Future<void> _set(String k, dynamic v) async {
    cfg[k] = v;
    if (mounted) setState(() {});
    try {
      await api.saveSettings(cfg);
    } catch (_) {}
  }

  Future<void> _saveAll() async {
    try {
      await api.saveSettings(cfg);
      if (mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          const SnackBar(content: Text('已保存, 引擎热重载中'), duration: Duration(seconds: 2)),
        );
      }
    } catch (_) {
      if (mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          const SnackBar(content: Text('保存失败: 引擎未响应'), duration: Duration(seconds: 2)),
        );
      }
    }
  }

  // ---- UI ----
  @override
  Widget build(BuildContext context) {
    final dark = Theme.of(context).brightness == Brightness.dark;
    return Scaffold(
      backgroundColor: Colors.transparent,
      body: Container(
        decoration: BoxDecoration(
          gradient: dark
              ? const LinearGradient(
                  begin: Alignment.topLeft,
                  end: Alignment.bottomRight,
                  colors: [Color(0xFF1C1E2C), Color(0xFF12131E)])
              : const LinearGradient(
                  begin: Alignment.topLeft,
                  end: Alignment.bottomRight,
                  colors: [Color(0xFFF0F3FA), Color(0xFFE8EDF7)]),
        ),
        child: SafeArea(
          child: Column(children: [
            _topbar(dark),
            Expanded(
              child: SingleChildScrollView(
                padding: const EdgeInsets.fromLTRB(24, 18, 24, 24),
                child: navIndex == 0 ? _statusPage(dark) : _placeholder(dark),
              ),
            ),
          ]),
        ),
      ),
    );
  }

  // ---- 顶部玻璃导航栏 ----
  Widget _topbar(bool dark) {
    final cs = Theme.of(context).colorScheme;
    final items = ['状态', '指针', '定位', '通用'];
    final icons = [
      Icons.wb_sunny_outlined,
      Icons.arrow_outward,
      Icons.location_on_outlined,
      Icons.settings_outlined
    ];
    return Padding(
      padding: const EdgeInsets.fromLTRB(14, 12, 14, 0),
      child: GlassContainer.card(
        child: DragToMoveArea(
          child: Padding(
            padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 10),
            child: Row(children: [
              Container(
                width: 30, height: 30, decoration: BoxDecoration(
                  shape: BoxShape.circle,
                  gradient: const LinearGradient(
                      begin: Alignment.topLeft,
                      end: Alignment.bottomRight,
                      colors: [Color(0xFFFFD76A), Color(0xFFE8933A)]),
                  boxShadow: [BoxShadow(color: cs.primary.withOpacity(.35), blurRadius: 10)]),
              ),
              const SizedBox(width: 10),
              Text('ThemeSwitcher2',
                  style: TextStyle(color: cs.onSurface, fontWeight: FontWeight.w600, fontSize: 14.5)),
              const SizedBox(width: 22),
              for (var i = 0; i < items.length; i++)
                Padding(
                  padding: const EdgeInsets.only(right: 6),
                  child: InkWell(
                    borderRadius: BorderRadius.circular(12),
                    onTap: () => setState(() => navIndex = i),
                    child: Container(
                      padding: const EdgeInsets.symmetric(horizontal: 14, vertical: 9),
                      decoration: BoxDecoration(
                        color: navIndex == i ? cs.primary.withOpacity(.14) : Colors.transparent,
                        borderRadius: BorderRadius.circular(12),
                        border: navIndex == i
                            ? Border.all(color: cs.primary.withOpacity(.35))
                            : null),
                      child: Row(children: [
                        Icon(icons[i], size: 16, color: navIndex == i ? cs.onSurface : cs.onSurface.withOpacity(.55)),
                        const SizedBox(width: 8),
                        Text(items[i],
                            style: TextStyle(
                                fontSize: 13.5,
                                color: navIndex == i ? cs.onSurface : cs.onSurface.withOpacity(.55))),
                      ]),
                    ),
                  ),
                ),
              const Spacer(),
              _engineChip(cs),
              const SizedBox(width: 10),
              _winBtn(Icons.minimize_rounded, () => windowManager.minimize()),
              _winBtn(Icons.crop_square_rounded, () async {
                if (await windowManager.isMaximized()) {
                  await windowManager.unmaximize();
                } else {
                  await windowManager.maximize();
                }
              }),
              _winBtn(Icons.close_rounded, () => windowManager.close()),
            ]),
          ),
        ),
      ),
    );
  }

  Widget _engineChip(ColorScheme cs) {
    final ok = engineOk;
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 6),
      decoration: BoxDecoration(
        borderRadius: BorderRadius.circular(999),
        color: ok ? Colors.green.withOpacity(.12) : Colors.red.withOpacity(.12),
        border: Border.all(color: ok ? Colors.green.withOpacity(.4) : Colors.red.withOpacity(.4))),
      child: Row(mainAxisSize: MainAxisSize.min, children: [
        Icon(ok ? Icons.check_circle : Icons.error_outline, size: 13,
            color: ok ? Colors.green : Colors.red),
        const SizedBox(width: 6),
        Text(ok ? '引擎运行中' : '引擎未响应',
            style: TextStyle(fontSize: 11.5, color: ok ? Colors.green : Colors.red)),
      ]),
    );
  }

  Widget _winBtn(IconData ic, VoidCallback onTap) {
    final cs = Theme.of(context).colorScheme;
    return InkWell(
      borderRadius: BorderRadius.circular(8),
      onTap: onTap,
      child: Padding(
        padding: const EdgeInsets.all(6),
        child: Icon(ic, size: 16, color: cs.onSurface.withOpacity(.7))),
    );
  }

  // ---- 状态页 ----
  Widget _statusPage(bool dark) {
    final cs = Theme.of(context).colorScheme;
    final modeColor = light ? Colors.green : Colors.orange;
    final modeText = light ? '浅色模式' : '深色模式';

    return Column(children: [
      // hero 状态卡
      GlassContainer.card(
        surfaceStyle: GlassSurfaceStyle.solid,
        child: Padding(
          padding: const EdgeInsets.all(22),
          child: Row(children: [
            Expanded(
              child: Column(crossAxisAlignment: CrossAxisAlignment.start, children: [
                Container(
                  padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 5),
                  decoration: BoxDecoration(
                    borderRadius: BorderRadius.circular(999),
                    color: modeColor.withOpacity(.13),
                    border: Border.all(color: modeColor.withOpacity(.45))),
                  child: Row(mainAxisSize: MainAxisSize.min, children: [
                    Container(width: 7, height: 7,
                        decoration: BoxDecoration(color: modeColor, shape: BoxShape.circle,
                            boxShadow: [BoxShadow(color: modeColor, blurRadius: 8)])),
                    const SizedBox(width: 7),
                    Text(modeText, style: TextStyle(fontSize: 12.5, color: modeColor)),
                  ]),
                ),
                const SizedBox(height: 12),
                Row(crossAxisAlignment: CrossAxisAlignment.end, children: [
                  Text('下次切换 $nextWhen',
                      style: TextStyle(fontSize: 26, fontWeight: FontWeight.w600,
                          color: cs.onSurface, letterSpacing: .2)),
                  const SizedBox(width: 8),
                  Padding(
                    padding: const EdgeInsets.only(bottom: 4),
                    child: Text('→ ${nextLight ? "浅色" : "深色"}',
                        style: TextStyle(fontSize: 15, color: cs.onSurface.withOpacity(.6))),
                  ),
                ]),
                const SizedBox(height: 16),
                Row(children: [
                  _meta('定位', location.isEmpty ? '获取中…' : location),
                  const SizedBox(width: 30),
                  _meta('日出', sunrise.isEmpty ? '…' : sunrise),
                  const SizedBox(width: 30),
                  _meta('日落', sunset.isEmpty ? '…' : sunset),
                ]),
              ]),
            ),
            const SizedBox(width: 18),
            // orb 日夜大按钮: 点击切到对面模式
            GestureDetector(
              onTap: () async {
                await api.setTheme(!light);
                setState(() => light = !light);
              },
              child: Container(
                width: 96, height: 96,
                decoration: BoxDecoration(
                  shape: BoxShape.circle,
                  gradient: !light
                      ? const LinearGradient(colors: [Color(0xFF7EA8FF), Color(0xFF2B4FD8)])
                      : const LinearGradient(colors: [Color(0xFFFFD76A), Color(0xFFE8933A)]),
                  boxShadow: [
                    BoxShadow(
                        color: !light
                            ? Colors.blue.withOpacity(.4)
                            : Colors.orange.withOpacity(.4),
                        blurRadius: 28, offset: const Offset(0, 10)),
                  ]),
                child: Icon(!light ? Icons.dark_mode_rounded : Icons.light_mode_rounded,
                    size: 40, color: Colors.white),
              ),
            ),
          ]),
        ),
      ),
      const SizedBox(height: 16),

      // 切换范围 + 偏移
      Row(crossAxisAlignment: CrossAxisAlignment.start, children: [
        Expanded(child: GlassContainer.card(
          surfaceStyle: GlassSurfaceStyle.solid,
          child: Padding(
            padding: const EdgeInsets.all(20),
            child: Column(crossAxisAlignment: CrossAxisAlignment.start, children: [
              _cardLabel(Icons.contrast_rounded, '「深色模式」切换哪些项'),
              _switchRow('应用主题', 'UWP / 资源管理器', b('switchApps'),
                  (v) => _set('switchApps', v)),
              _switchRow('系统主题', '任务栏 / 开始菜单', b('switchSystem'),
                  (v) => _set('switchSystem', v)),
            ]),
          ),
        )),
        const SizedBox(width: 16),
        Expanded(child: GlassContainer.card(
          surfaceStyle: GlassSurfaceStyle.solid,
          child: Padding(
            padding: const EdgeInsets.all(20),
            child: Column(crossAxisAlignment: CrossAxisAlignment.start, children: [
              _cardLabel(Icons.schedule_rounded, '切换时机偏移(分钟)'),
              _stepperRow('日出', i('sunriseOffsetMin'), (v) => _set('sunriseOffsetMin', v)),
              _stepperRow('日落', i('sunsetOffsetMin'), (v) => _set('sunsetOffsetMin', v)),
            ]),
          ),
        )),
      ]),
      const SizedBox(height: 16),

      // 指针卡
      GlassContainer.card(
        surfaceStyle: GlassSurfaceStyle.solid,
        child: Padding(
          padding: const EdgeInsets.all(20),
          child: Column(crossAxisAlignment: CrossAxisAlignment.start, children: [
            _cardLabel(Icons.arrow_outward_rounded, '鼠标指针 · 同步切换'),
            Row(crossAxisAlignment: CrossAxisAlignment.start, children: [
              Expanded(child: Column(crossAxisAlignment: CrossAxisAlignment.start, children: [
                Text('浅色方案', style: TextStyle(fontSize: 13, color: cs.onSurface.withOpacity(.65))),
                const SizedBox(height: 6),
                _schemePicker('cursorLightScheme'),
              ])),
              const SizedBox(width: 18),
              Expanded(child: Column(crossAxisAlignment: CrossAxisAlignment.start, children: [
                Text('深色方案', style: TextStyle(fontSize: 13, color: cs.onSurface.withOpacity(.65))),
                const SizedBox(height: 6),
                _schemePicker('cursorDarkScheme'),
              ])),
            ]),
            const SizedBox(height: 8),
            _switchRow('跟随主题自动切换指针', '识别不到当前方案时保持现状', b('switchCursor'),
                (v) => _set('switchCursor', v)),
          ]),
        ),
      ),
      const SizedBox(height: 16),

      // 底部条
      GlassContainer.card(
        surfaceStyle: GlassSurfaceStyle.solid,
        child: Padding(
          padding: const EdgeInsets.symmetric(horizontal: 20, vertical: 14),
          child: Row(children: [
            Column(crossAxisAlignment: CrossAxisAlignment.start, children: [
              Text('开机自启', style: TextStyle(fontSize: 14, color: cs.onSurface)),
              Text('HKCU Run', style: TextStyle(fontSize: 11.5, color: cs.onSurface.withOpacity(.55))),
            ]),
            const Spacer(),
            TextButton(onPressed: _openConfig, child: const Text('打开配置文件')),
            const SizedBox(width: 8),
            FilledButton(onPressed: _saveAll, child: const Text('保存并关闭')),
          ]),
        ),
      ),
    ]);
  }

  Widget _meta(String k, String v) {
    final cs = Theme.of(context).colorScheme;
    return Column(crossAxisAlignment: CrossAxisAlignment.start, children: [
      Text(k, style: TextStyle(fontSize: 12, color: cs.onSurface.withOpacity(.55))),
      const SizedBox(height: 4),
      Text(v, style: TextStyle(fontSize: 14, fontWeight: FontWeight.w600, color: cs.onSurface)),
    ]);
  }

  Widget _cardLabel(IconData ic, String text) {
    final cs = Theme.of(context).colorScheme;
    return Padding(
      padding: const EdgeInsets.only(bottom: 10),
      child: Row(children: [
        Icon(ic, size: 15, color: cs.onSurface.withOpacity(.55)),
        const SizedBox(width: 8),
        Text(text, style: TextStyle(fontSize: 12.5, color: cs.onSurface.withOpacity(.55))),
      ]),
    );
  }

  Widget _switchRow(String title, String desc, bool value, ValueChanged<bool> onChanged) {
    final cs = Theme.of(context).colorScheme;
    return Padding(
      padding: const EdgeInsets.symmetric(vertical: 8),
      child: Row(children: [
        Expanded(child: Column(crossAxisAlignment: CrossAxisAlignment.start, children: [
          Text(title, style: TextStyle(fontSize: 14, color: cs.onSurface)),
          const SizedBox(height: 3),
          Text(desc, style: TextStyle(fontSize: 11.5, color: cs.onSurface.withOpacity(.55))),
        ])),
        GlassSwitch(value: value, onChanged: onChanged),
      ]),
    );
  }

  Widget _stepperRow(String title, int value, ValueChanged<int> onChanged) {
    final cs = Theme.of(context).colorScheme;
    return Padding(
      padding: const EdgeInsets.symmetric(vertical: 8),
      child: Row(children: [
        Expanded(child: Text(title, style: TextStyle(fontSize: 14, color: cs.onSurface))),
        Container(
          padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 4),
          decoration: BoxDecoration(
            borderRadius: BorderRadius.circular(12),
            color: cs.onSurface.withOpacity(.06),
            border: Border.all(color: cs.onSurface.withOpacity(.12))),
          child: Row(children: [
            Text('$value', style: TextStyle(fontSize: 14, fontWeight: FontWeight.w600, color: cs.onSurface)),
            const SizedBox(width: 10),
            Column(children: [
              InkWell(onTap: () => onChanged(value + 1), child: Icon(Icons.arrow_drop_up, size: 18, color: cs.onSurface)),
              InkWell(onTap: () => onChanged(value - 1), child: Icon(Icons.arrow_drop_down, size: 18, color: cs.onSurface)),
            ]),
          ]),
        ),
      ]),
    );
  }

  Widget _schemePicker(String key) {
    final cs = Theme.of(context).colorScheme;
    final current = str(key);
    final items = [...schemes];
    if (current.isNotEmpty && !items.contains(current)) items.insert(0, current);
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 14),
      decoration: BoxDecoration(
        borderRadius: BorderRadius.circular(12),
        color: cs.onSurface.withOpacity(.06),
        border: Border.all(color: cs.onSurface.withOpacity(.12))),
      child: DropdownButtonHideUnderline(
        child: DropdownButton<String>(
          value: current.isEmpty ? null : current,
          isExpanded: true,
          hint: Text('加载中…', style: TextStyle(fontSize: 13.5, color: cs.onSurface.withOpacity(.6))),
          dropdownColor: dark() ? const Color(0xFF262A36) : Colors.white,
          items: [for (final s in items) DropdownMenuItem(value: s, child: Text(s, style: TextStyle(fontSize: 13.5, color: cs.onSurface)))],
          onChanged: (v) { if (v != null) _set(key, v); },
        ),
      ),
    );
  }

  bool dark() => Theme.of(context).brightness == Brightness.dark;

  Future<void> _openConfig() async {
    // 引擎侧 INI; 记事本打开(路径由引擎约定)
    await api.reload();
  }

  Widget _placeholder(bool dark) {
    final cs = Theme.of(context).colorScheme;
    return GlassContainer.card(
      surfaceStyle: GlassSurfaceStyle.solid,
      child: Padding(
        padding: const EdgeInsets.all(40),
        child: Center(child: Text('这一页在 v2 到来', style: TextStyle(color: cs.onSurface.withOpacity(.6)))),
      ),
    );
  }
}
