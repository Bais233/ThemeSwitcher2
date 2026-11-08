// api.dart — 引擎控制接口客户端(127.0.0.1:27125)
import 'dart:async';
import 'dart:convert';
import 'package:http/http.dart' as http;

class EngineApi {
  static const base = 'http://127.0.0.1:27125';

  Future<Map<String, dynamic>> status() async {
    final r = await http.get(Uri.parse('$base/status')).timeout(const Duration(seconds: 3));
    return jsonDecode(r.body) as Map<String, dynamic>;
  }

  Future<Map<String, dynamic>> settings() async {
    final r = await http.get(Uri.parse('$base/settings')).timeout(const Duration(seconds: 3));
    return jsonDecode(r.body) as Map<String, dynamic>;
  }

  Future<void> saveSettings(Map<String, dynamic> body) async {
    await http
        .post(Uri.parse('$base/settings'),
            headers: {'Content-Type': 'application/json'}, body: jsonEncode(body))
        .timeout(const Duration(seconds: 3));
  }

  Future<void> setTheme(bool light) async {
    await http
        .post(Uri.parse('$base/theme/${light ? 'light' : 'dark'}'))
        .timeout(const Duration(seconds: 3));
  }

  Future<void> reload() async {
    await http.post(Uri.parse('$base/reload')).timeout(const Duration(seconds: 3));
  }

  Future<List<String>> schemes() async {
    final r = await http.get(Uri.parse('$base/schemes')).timeout(const Duration(seconds: 3));
    final list = (jsonDecode(r.body)['schemes'] as List).cast<String>();
    return list;
  }
}
