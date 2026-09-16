import 'dart:async';
import '../models/connection_state.dart';
import '../models/remote_packet.dart';

abstract class BaseTransport {
  final StreamController<ConnectionStatus> statusController =
      StreamController<ConnectionStatus>.broadcast();

  Stream<ConnectionStatus> get onStatusChanged => statusController.stream;

  Future<bool> connect();
  Future<void> disconnect();
  bool send(RemotePacket packet, {int? bleByteMask});
  bool get isConnected;
}