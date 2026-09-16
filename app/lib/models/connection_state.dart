enum TransportMedium { wifi, ble, none }

class ConnectionStatus {
  final TransportMedium medium;
  final String label;

  const ConnectionStatus({required this.medium, required this.label});

  factory ConnectionStatus.disconnected() => const ConnectionStatus(
        medium: TransportMedium.none,
        label: 'Disconnected',
      );

  factory ConnectionStatus.wifi() => const ConnectionStatus(
        medium: TransportMedium.wifi,
        label: 'Wi-Fi (ZeroConf)',
      );

  factory ConnectionStatus.ble() => const ConnectionStatus(
        medium: TransportMedium.ble,
        label: 'Bluetooth HID',
      );
}