import 'dart:async';
import '../models/connection_state.dart';
import '../models/remote_packet.dart';
import 'ble_transport.dart';
import 'websocket_transport.dart';

class TransportCoordinator {
  final WebSocketTransport _ws = WebSocketTransport();
  final BleTransport _ble = BleTransport();
  int _seq = 1;

  final StreamController<ConnectionStatus> _statusController =
      StreamController<ConnectionStatus>.broadcast();

  Stream<ConnectionStatus> get onStatusChanged => _statusController.stream;

  Future<void> init() async {
    _statusController.add(ConnectionStatus.disconnected());

    _ws.onStatusChanged.listen((status) {
      if (status.medium == TransportMedium.wifi) {
        _statusController.add(status);
      } else {
        // WiFi lost -> Attempt BLE failover
        _fallbackToBle();
      }
    });

    _ble.onStatusChanged.listen((status) {
      if (!_ws.isConnected) {
        _statusController.add(status);
      }
    });

    final wifiOk = await _ws.connect();
    if (!wifiOk) {
      await _fallbackToBle();
    }
  }

  Future<void> _fallbackToBle() async {
    if (!_ble.isConnected) {
      await _ble.connect();
    }
  }

  void dispatch(RemoteCommand cmd, {int? bleByteMask}) {
    final packet = RemotePacket(seq: _seq++, command: cmd);

    if (_ws.isConnected) {
      _ws.send(packet);
    } else if (_ble.isConnected && bleByteMask != null) {
      _ble.send(packet, bleByteMask: bleByteMask);
    }
  }
}