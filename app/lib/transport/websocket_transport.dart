import 'dart:async';
import 'package:nsd/nsd.dart';
import 'package:web_socket_channel/web_socket_channel.dart';
import '../core/constants/network_constants.dart';
import '../models/connection_state.dart';
import '../models/remote_packet.dart';
import 'base_transport.dart';

class WebSocketTransport extends BaseTransport {
  WebSocketChannel? _channel;
  bool _connected = false;

  @override
  bool get isConnected => _connected;

  // Sıfır hardcode: mDNS ile otomatik bulmayı dener
  @override
  Future<bool> connect() async {
    final completer = Completer<bool>();

    try {
      final discovery = await startDiscovery(NetworkConstants.mdnsServiceType);
      discovery.addListener(() async {
        if (discovery.services.isNotEmpty && !_connected) {
          final s = discovery.services.first;
          if (s.host != null && s.port != null) {
            final uri = 'ws://${s.host}:${s.port}${NetworkConstants.wsPath}';
            final success = await connectWithHost(s.host!, s.port ?? NetworkConstants.defaultPort);
            stopDiscovery(discovery);
            if (!completer.isCompleted) completer.complete(success);
          }
        }
      });

      // 4 saniye mDNS yanıt vermezse vazgeç
      Future.delayed(const Duration(seconds: 4), () {
        if (!completer.isCompleted) completer.complete(false);
      });
    } catch (_) {
      if (!completer.isCompleted) completer.complete(false);
    }

    return completer.future;
  }

  // QR koddan veya manuel diyalogdan gelen hedef adrese bağlanır
  Future<bool> connectWithHost(String host, int port) async {
    final uri = 'ws://$host:$port${NetworkConstants.wsPath}';
    try {
      await _channel?.sink.close();
      _channel = WebSocketChannel.connect(Uri.parse(uri));
      await _channel!.ready;
      _connected = true;
      statusController.add(ConnectionStatus.wifi());

      _channel!.stream.listen(
        (_) {},
        onDone: () {
          _connected = false;
          statusController.add(ConnectionStatus.disconnected());
        },
        onError: (_) {
          _connected = false;
          statusController.add(ConnectionStatus.disconnected());
        },
      );
      return true;
    } catch (_) {
      _connected = false;
      statusController.add(ConnectionStatus.disconnected());
      return false;
    }
  }

  @override
  bool send(RemotePacket packet, {int? bleByteMask}) {
    if (_connected && _channel != null) {
      _channel!.sink.add(packet.serialize());
      return true;
    }
    return false;
  }

  @override
  Future<void> disconnect() async {
    _connected = false;
    await _channel?.sink.close();
    statusController.add(ConnectionStatus.disconnected());
  }
}