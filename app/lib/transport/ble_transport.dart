import 'dart:async';
import 'package:flutter_blue_plus/flutter_blue_plus.dart';
import '../core/constants/ble_constants.dart';
import '../models/connection_state.dart';
import '../models/remote_packet.dart';
import 'base_transport.dart';

class BleTransport extends BaseTransport {
  BluetoothDevice? _device;
  BluetoothCharacteristic? _reportChar;
  bool _connected = false;

  @override
  bool get isConnected => _connected;

  @override
  Future<bool> connect() async {
    final completer = Completer<bool>();

    try {
      FlutterBluePlus.startScan(timeout: const Duration(seconds: 4));
      FlutterBluePlus.scanResults.listen((results) async {
        for (ScanResult r in results) {
          if (r.advertisementData.advName == BleConstants.targetDeviceName) {
            await FlutterBluePlus.stopScan();
            _device = r.device;
            await _device!.connect(license: License.free);

            List<BluetoothService> services = await _device!.discoverServices();
            for (var s in services) {
              if (s.uuid == Guid(BleConstants.hidServiceUuid)) {
                for (var c in s.characteristics) {
                  if (c.uuid == Guid(BleConstants.reportCharUuid)) {
                    _reportChar = c;
                    _connected = true;
                    statusController.add(ConnectionStatus.ble());
                    if (!completer.isCompleted) completer.complete(true);
                    return;
                  }
                }
              }
            }
          }
        }
      });

      Future.delayed(const Duration(seconds: 4), () {
        if (!completer.isCompleted) completer.complete(false);
      });
    } catch (_) {
      if (!completer.isCompleted) completer.complete(false);
    }

    return completer.future;
  }

  @override
  bool send(RemotePacket packet, {int? bleByteMask}) {
    if (_connected && _reportChar != null && bleByteMask != null) {
      _reportChar!.write([bleByteMask], withoutResponse: true);
      return true;
    }
    return false;
  }

  @override
  Future<void> disconnect() async {
    _connected = false;
    await _device?.disconnect();
    statusController.add(ConnectionStatus.disconnected());
  }
}