import 'package:flutter/material.dart';
import 'package:provider/provider.dart';
import 'core/theme/app_theme.dart';
import 'ui/controllers/remote_controller.dart';
import 'ui/screens/remote_screen.dart';

void main() {
  WidgetsFlutterBinding.ensureInitialized();
  runApp(
    ChangeNotifierProvider(
      create: (_) => RemoteController(),
      child: const TiVarchRemoteApp(),
    ),
  );
}

class TiVarchRemoteApp extends StatelessWidget {
  const TiVarchRemoteApp({super.key});

  @override
  Widget build(BuildContext context) {
    return MaterialApp(
      debugShowCheckedModeBanner: false,
      theme: AppTheme.darkTheme,
      home: const RemoteScreen(),
    );
  }
}