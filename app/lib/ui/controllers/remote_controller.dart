import 'package:flutter/foundation.dart';
import '../../core/constants/ble_constants.dart';
import '../../core/services/haptic_service.dart';
import '../../models/connection_state.dart';
import '../../models/remote_packet.dart';
import '../../transport/transport_coordinator.dart';

class RemoteController extends ChangeNotifier {
  final TransportCoordinator coordinator = TransportCoordinator();
  ConnectionStatus status = ConnectionStatus.disconnected();

  RemoteController() {
    coordinator.onStatusChanged.listen((s) {
      status = s;
      notifyListeners();
    });
    coordinator.init();
  }

  void sendDPad(String action, String state, {int? bleMask}) {
    HapticService.light();
    coordinator.dispatch(
      RemoteCommand(type: 'd_pad', payload: {'action': action, 'state': state}),
      bleByteMask: bleMask,
    );
  }

  void sendMedia(String action, {int? bleMask}) {
    HapticService.medium();
    coordinator.dispatch(
      RemoteCommand(type: 'media', payload: action),
      bleByteMask: bleMask,
    );
  }

  void sendText(String text) {
    HapticService.light();
    coordinator.dispatch(
      RemoteCommand(type: 'keyboard', payload: {'text': text}),
    );
  }

  void sendAppLaunch(String appId) {
    HapticService.heavy();
    coordinator.dispatch(
      RemoteCommand(type: 'launch_app', payload: {'app_id': appId}),
    );
  }

  void sendPower(String action) {
    HapticService.heavy();
    coordinator.dispatch(
      RemoteCommand(type: 'power', payload: action),
    );
  }
}