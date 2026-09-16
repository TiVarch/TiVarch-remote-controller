import 'dart:convert';

class RemoteCommand {
  final String type;
  final dynamic payload;

  RemoteCommand({required this.type, this.payload});

  Map<String, dynamic> toJson() => {
        'type': type,
        if (payload != null) 'payload': payload,
      };
}

class RemotePacket {
  final int seq;
  final RemoteCommand command;

  RemotePacket({required this.seq, required this.command});

  String serialize() => jsonEncode({
        'seq': seq,
        'command': command.toJson(),
      });
}