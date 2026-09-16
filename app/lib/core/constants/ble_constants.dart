class BleConstants {
  static const String hidServiceUuid = "00001812-0000-1000-8000-00805f9b34fb";
  static const String reportCharUuid = "00002a4d-0000-1000-8000-00805f9b34fb";
  static const String targetDeviceName = "TiVarch Remote";

  // HID Consumer Bitmask flags matching Rust daemon
  static const int maskVolumeUp = 0x01;
  static const int maskVolumeDown = 0x02;
  static const int maskMute = 0x04;
  static const int maskPlayPause = 0x08;
  static const int maskBack = 0x10;
  static const int maskHome = 0x40;
}