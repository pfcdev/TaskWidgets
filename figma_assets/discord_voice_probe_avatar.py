from __future__ import annotations

import argparse
import json
import os
import struct
import sys
import time
import uuid
from dataclasses import dataclass
from typing import Any, Callable

try:
    import pywintypes
    import win32file
    import win32pipe
except ImportError as exc:
    raise SystemExit(
        "Eksik paket: pywin32\nKurulum: py -m pip install pywin32 requests"
    ) from exc

try:
    import requests
except ImportError as exc:
    raise SystemExit(
        "Eksik paket: requests\nKurulum: py -m pip install pywin32 requests"
    ) from exc


OP_HANDSHAKE = 0
OP_FRAME = 1
OP_CLOSE = 2
OP_PING = 3
OP_PONG = 4

OP_NAMES = {
    OP_HANDSHAKE: "HANDSHAKE",
    OP_FRAME: "FRAME",
    OP_CLOSE: "CLOSE",
    OP_PING: "PING",
    OP_PONG: "PONG",
}

CHANNEL_EVENTS = (
    "VOICE_STATE_CREATE",
    "VOICE_STATE_UPDATE",
    "VOICE_STATE_DELETE",
    "SPEAKING_START",
    "SPEAKING_STOP",
)


class DiscordIPCError(RuntimeError):
    pass


class DiscordIPC:
    def __init__(self, raw_output: bool = False) -> None:
        self.handle: Any | None = None
        self.pipe_path: str | None = None
        self.raw_output = raw_output

    def connect(self) -> str:
        errors: list[str] = []
        for index in range(10):
            path = rf"\\?\pipe\discord-ipc-{index}"
            try:
                handle = win32file.CreateFile(
                    path,
                    win32file.GENERIC_READ | win32file.GENERIC_WRITE,
                    0,
                    None,
                    win32file.OPEN_EXISTING,
                    0,
                    None,
                )
                # Discord IPC byte-stream framing kullanır.
                win32pipe.SetNamedPipeHandleState(
                    handle, win32pipe.PIPE_READMODE_BYTE, None, None
                )
                self.handle = handle
                self.pipe_path = path
                return path
            except pywintypes.error as exc:
                errors.append(f"{path}: {exc.winerror}")

        raise DiscordIPCError(
            "Discord IPC pipe bulunamadı. Discord masaüstü uygulamasının açık "
            "olduğundan emin ol. Denenen yollar:\n  " + "\n  ".join(errors)
        )

    def close(self) -> None:
        if self.handle is not None:
            try:
                win32file.CloseHandle(self.handle)
            finally:
                self.handle = None

    def _require_handle(self) -> Any:
        if self.handle is None:
            raise DiscordIPCError("IPC bağlantısı açık değil.")
        return self.handle

    def _read_exactly(self, byte_count: int) -> bytes:
        handle = self._require_handle()
        chunks: list[bytes] = []
        remaining = byte_count

        while remaining:
            try:
                result, data = win32file.ReadFile(handle, remaining)
            except pywintypes.error as exc:
                raise DiscordIPCError(
                    f"Pipe okunamadı: WinError {exc.winerror}: {exc.strerror}"
                ) from exc

            # ERROR_MORE_DATA (234), message-mode pipe'larda kısmi veri demektir.
            if result not in (0, 234):
                raise DiscordIPCError(f"Pipe okuma hatası: {result}")
            if not data:
                raise DiscordIPCError("Discord IPC bağlantıyı kapattı.")

            chunks.append(data)
            remaining -= len(data)

        return b"".join(chunks)

    def send_raw(self, opcode: int, payload_bytes: bytes) -> None:
        handle = self._require_handle()
        packet = struct.pack("<II", opcode, len(payload_bytes)) + payload_bytes
        try:
            result, _ = win32file.WriteFile(handle, packet)
        except pywintypes.error as exc:
            raise DiscordIPCError(
                f"Pipe'a yazılamadı: WinError {exc.winerror}: {exc.strerror}"
            ) from exc
        if result != 0:
            raise DiscordIPCError(f"Pipe yazma hatası: {result}")

    def send_json(self, opcode: int, payload: dict[str, Any]) -> None:
        body = json.dumps(payload, ensure_ascii=False, separators=(",", ":")).encode(
            "utf-8"
        )
        if self.raw_output:
            print_packet("GÖNDER", opcode, payload)
        self.send_raw(opcode, body)

    def receive(self) -> tuple[int, Any]:
        while True:
            header = self._read_exactly(8)
            opcode, length = struct.unpack("<II", header)
            body = self._read_exactly(length) if length else b""

            if opcode == OP_PING:
                # Discord'un gönderdiği ping gövdesini aynen pong olarak geri yolla.
                self.send_raw(OP_PONG, body)
                if self.raw_output:
                    print_packet("AL/PONG", opcode, decode_body(body))
                continue

            payload = decode_body(body)
            if self.raw_output:
                print_packet("AL", opcode, payload)
            return opcode, payload

    def command(
        self,
        command: str,
        args: dict[str, Any] | None = None,
        event: str | None = None,
    ) -> str:
        nonce = str(uuid.uuid4())
        payload: dict[str, Any] = {
            "cmd": command,
            "args": args or {},
            "nonce": nonce,
        }
        if event is not None:
            payload["evt"] = event
        self.send_json(OP_FRAME, payload)
        return nonce

    def wait_for_nonce(
        self,
        nonce: str,
        dispatch_handler: Callable[[dict[str, Any]], None] | None = None,
    ) -> dict[str, Any]:
        while True:
            opcode, payload = self.receive()
            if opcode == OP_CLOSE:
                raise DiscordIPCError(f"Discord bağlantıyı kapattı: {payload}")
            if not isinstance(payload, dict):
                continue

            if payload.get("cmd") == "DISPATCH" and dispatch_handler:
                dispatch_handler(payload)

            if payload.get("nonce") == nonce:
                return payload


def decode_body(body: bytes) -> Any:
    if not body:
        return None
    try:
        return json.loads(body.decode("utf-8"))
    except (UnicodeDecodeError, json.JSONDecodeError):
        return {"_raw_hex": body.hex(), "_raw_repr": repr(body)}


def print_packet(direction: str, opcode: int, payload: Any) -> None:
    stamp = time.strftime("%H:%M:%S")
    opcode_name = OP_NAMES.get(opcode, str(opcode))
    print(f"\n[{stamp}] {direction} {opcode_name}")
    print(json.dumps(payload, ensure_ascii=False, indent=2))


def rpc_error(payload: dict[str, Any]) -> tuple[int | None, str] | None:
    if payload.get("evt") != "ERROR":
        return None
    data = payload.get("data") or {}
    return data.get("code"), data.get("message", "Bilinmeyen RPC hatası")


def require_success(payload: dict[str, Any], operation: str) -> dict[str, Any]:
    error = rpc_error(payload)
    if error:
        code, message = error
        raise DiscordIPCError(f"{operation} başarısız — {code}: {message}")
    return payload


def avatar_url(user: dict[str, Any]) -> str | None:
    """Discord user nesnesinden 128px avatar CDN adresi üretir."""
    user_id = user.get("id")
    avatar = user.get("avatar")

    if not user_id:
        return None

    if avatar:
        extension = "gif" if str(avatar).startswith("a_") else "png"
        return (
            f"https://cdn.discordapp.com/avatars/"
            f"{user_id}/{avatar}.{extension}?size=128"
        )

    # Yeni kullanıcı adı sisteminde varsayılan avatar indeksi.
    default_index = (int(user_id) >> 22) % 6
    return f"https://cdn.discordapp.com/embed/avatars/{default_index}.png"


@dataclass
class ChannelState:
    channel_id: str | None = None
    channel_name: str | None = None
    users: dict[str, dict[str, Any]] | None = None
    speaking: set[str] | None = None

    def __post_init__(self) -> None:
        self.users = {}
        self.speaking = set()


class VoiceMonitor:
    def __init__(self, ipc: DiscordIPC) -> None:
        self.ipc = ipc
        self.state = ChannelState()
        self.pending: dict[str, str] = {}

    def send_tracked(
        self,
        command: str,
        args: dict[str, Any] | None = None,
        event: str | None = None,
        purpose: str | None = None,
    ) -> str:
        nonce = self.ipc.command(command, args, event)
        self.pending[nonce] = purpose or f"{command}:{event or ''}"
        return nonce

    def subscribe_global_events(self) -> None:
        self.send_tracked(
            "SUBSCRIBE", {}, "VOICE_CHANNEL_SELECT", "subscribe_global"
        )
        self.send_tracked(
            "SUBSCRIBE", {}, "VOICE_CONNECTION_STATUS", "subscribe_connection"
        )

    def request_selected_channel(self) -> None:
        self.send_tracked(
            "GET_SELECTED_VOICE_CHANNEL", {}, purpose="get_selected_channel"
        )

    def _unsubscribe_channel(self, channel_id: str) -> None:
        for event in CHANNEL_EVENTS:
            self.send_tracked(
                "UNSUBSCRIBE",
                {"channel_id": channel_id},
                event,
                f"unsubscribe:{event}",
            )

    def _subscribe_channel(self, channel_id: str) -> None:
        for event in CHANNEL_EVENTS:
            self.send_tracked(
                "SUBSCRIBE",
                {"channel_id": channel_id},
                event,
                f"subscribe:{event}",
            )

    def set_channel(self, channel: dict[str, Any] | None) -> None:
        old_channel_id = self.state.channel_id
        new_channel_id = str(channel.get("id")) if channel and channel.get("id") else None

        if old_channel_id and old_channel_id != new_channel_id:
            self._unsubscribe_channel(old_channel_id)

        self.state.channel_id = new_channel_id
        self.state.channel_name = channel.get("name") if channel else None
        self.state.users = {}
        self.state.speaking = set()

        if not channel or not new_channel_id:
            print("\n[SES] Şu anda seçili/bağlı bir ses kanalı yok.")
            return

        for voice_state in channel.get("voice_states") or []:
            self.upsert_voice_user(voice_state, announce=False)

        print(f"\n[SES] Kanal: {self.state.channel_name!r} ({new_channel_id})")
        if self.state.users:
            print("[SES] Kanaldaki kullanıcılar:")
            for user_id, entry in self.state.users.items():
                print(
                    f"  - {entry['display_name']} | id={user_id} | "
                    f"mute={entry.get('mute')} | avatar={entry.get('avatar_url')}"
                )
        else:
            print("[SES] Kanal kullanıcı listesi boş döndü.")

        if old_channel_id != new_channel_id:
            self._subscribe_channel(new_channel_id)

    def upsert_voice_user(self, data: dict[str, Any], announce: bool = True) -> None:
        user = data.get("user") or {}
        user_id = str(user.get("id") or data.get("user_id") or "")
        if not user_id:
            return

        voice_state = data.get("voice_state") or {}
        display_name = (
            data.get("nick")
            or user.get("global_name")
            or user.get("username")
            or user_id
        )
        entry = {
            "display_name": display_name,
            "username": user.get("username"),
            "global_name": user.get("global_name"),
            "nick": data.get("nick"),
            "avatar_url": avatar_url(user),
            "bot": user.get("bot", False),
            "mute": data.get("mute", voice_state.get("mute")),
            "deaf": voice_state.get("deaf"),
            "self_mute": voice_state.get("self_mute"),
            "self_deaf": voice_state.get("self_deaf"),
            "suppress": voice_state.get("suppress"),
            "volume": data.get("volume"),
        }
        assert self.state.users is not None
        self.state.users[user_id] = entry

        if announce:
            print(
                f"[VOICE_STATE] {display_name} | id={user_id} | "
                f"self_mute={entry['self_mute']} | self_deaf={entry['self_deaf']}"
            )

    def remove_voice_user(self, data: dict[str, Any]) -> None:
        user = data.get("user") or {}
        user_id = str(user.get("id") or data.get("user_id") or "")
        if not user_id:
            return
        assert self.state.users is not None
        assert self.state.speaking is not None
        entry = self.state.users.pop(user_id, None)
        self.state.speaking.discard(user_id)
        print(f"[AYRILDI] {(entry or {}).get('display_name', user_id)} | id={user_id}")

    def handle_dispatch(self, payload: dict[str, Any]) -> None:
        event = payload.get("evt")
        data = payload.get("data") or {}

        if event == "VOICE_CHANNEL_SELECT":
            print(
                f"[KANAL DEĞİŞTİ] channel_id={data.get('channel_id')} "
                f"guild_id={data.get('guild_id')}"
            )
            self.request_selected_channel()
            return

        if event in ("VOICE_STATE_CREATE", "VOICE_STATE_UPDATE"):
            self.upsert_voice_user(data)
            return

        if event == "VOICE_STATE_DELETE":
            self.remove_voice_user(data)
            return

        if event in ("SPEAKING_START", "SPEAKING_STOP"):
            user_id = str(data.get("user_id") or "")
            assert self.state.users is not None
            assert self.state.speaking is not None
            entry = self.state.users.get(user_id, {})
            display_name = entry.get("display_name", user_id)
            avatar = entry.get("avatar_url")

            if event == "SPEAKING_START":
                self.state.speaking.add(user_id)
                symbol = "KONUŞUYOR"
            else:
                self.state.speaking.discard(user_id)
                symbol = "DURDU"

            print(
                f"[{symbol}] {display_name} | id={user_id} | "
                f"avatar={avatar or '-'}"
            )
            return

        if event == "VOICE_CONNECTION_STATUS":
            print(
                f"[BAĞLANTI] state={data.get('state')} "
                f"ping={data.get('last_ping')}ms avg={data.get('average_ping')}ms"
            )

    def handle_response(self, payload: dict[str, Any]) -> None:
        nonce = payload.get("nonce")
        purpose = self.pending.pop(str(nonce), None) if nonce else None

        error = rpc_error(payload)
        if error:
            code, message = error
            print(f"[RPC HATA] {purpose or payload.get('cmd')} — {code}: {message}")
            return

        if purpose == "get_selected_channel":
            data = payload.get("data")
            self.set_channel(data if isinstance(data, dict) else None)

    def run(self) -> None:
        self.subscribe_global_events()
        self.request_selected_channel()
        print("\nGerçek zamanlı dinleme başladı. Çıkmak için Ctrl+C.\n")

        while True:
            opcode, payload = self.ipc.receive()
            if opcode == OP_CLOSE:
                raise DiscordIPCError(f"Discord bağlantıyı kapattı: {payload}")
            if not isinstance(payload, dict):
                continue

            if payload.get("cmd") == "DISPATCH":
                self.handle_dispatch(payload)
            else:
                self.handle_response(payload)


def authorize_and_exchange(
    ipc: DiscordIPC,
    client_id: str,
    client_secret: str,
    redirect_uri: str,
) -> str:
    scopes = ["rpc", "identify", "rpc.voice.read"]
    print("\nDiscord istemcisinde izin penceresi açılması bekleniyor...")
    nonce = ipc.command(
        "AUTHORIZE",
        {
            "client_id": client_id,
            "scopes": scopes,
        },
    )
    response = require_success(ipc.wait_for_nonce(nonce), "AUTHORIZE")
    code = (response.get("data") or {}).get("code")
    if not code:
        raise DiscordIPCError(f"AUTHORIZE cevabında code yok: {response}")

    token_response = requests.post(
        "https://discord.com/api/v10/oauth2/token",
        data={
            "client_id": client_id,
            "client_secret": client_secret,
            "grant_type": "authorization_code",
            "code": code,
            "redirect_uri": redirect_uri,
        },
        timeout=20,
    )

    try:
        token_json = token_response.json()
    except requests.JSONDecodeError as exc:
        raise DiscordIPCError(
            f"Token endpoint JSON döndürmedi: HTTP {token_response.status_code}"
        ) from exc

    if not token_response.ok or "access_token" not in token_json:
        safe_error = {
            key: value
            for key, value in token_json.items()
            if key not in {"access_token", "refresh_token"}
        }
        raise DiscordIPCError(
            f"Token değişimi başarısız: HTTP {token_response.status_code}: "
            f"{json.dumps(safe_error, ensure_ascii=False)}"
        )

    print("Yetkilendirme tamamlandı; access token yalnızca bellekte tutuluyor.")
    return str(token_json["access_token"])


def authenticate(ipc: DiscordIPC, access_token: str) -> dict[str, Any]:
    nonce = ipc.command("AUTHENTICATE", {"access_token": access_token})
    response = require_success(ipc.wait_for_nonce(nonce), "AUTHENTICATE")
    data = response.get("data") or {}
    user = data.get("user") or {}
    print(
        f"\nGiriş tamam: {user.get('global_name') or user.get('username')} "
        f"({user.get('id')})"
    )
    print(f"İzin verilen scope'lar: {', '.join(data.get('scopes') or [])}")
    return response


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Windows Discord istemcisinin yerel IPC/RPC ses olaylarını gösterir."
    )
    parser.add_argument(
        "--client-id",
        default=os.getenv("DISCORD_CLIENT_ID"),
        help="Discord Application/Client ID (veya DISCORD_CLIENT_ID)",
    )
    parser.add_argument(
        "--access-token",
        default=os.getenv("DISCORD_ACCESS_TOKEN"),
        help="Önceden alınmış OAuth access token (veya DISCORD_ACCESS_TOKEN)",
    )
    parser.add_argument(
        "--authorize",
        action="store_true",
        help="Discord istemcisinde izin iste ve token'ı otomatik al",
    )
    parser.add_argument(
        "--client-secret",
        default=os.getenv("DISCORD_CLIENT_SECRET"),
        help="Sadece --authorize testi için (veya DISCORD_CLIENT_SECRET)",
    )
    parser.add_argument(
        "--redirect-uri",
        default=os.getenv("DISCORD_REDIRECT_URI"),
        help="Developer Portal'da kayıtlı redirect URI",
    )
    parser.add_argument(
        "--raw",
        action="store_true",
        help="Gelen/giden bütün IPC JSON paketlerini yazdır",
    )
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    if sys.platform != "win32":
        print("Bu örnek Windows named pipe için yazıldı.", file=sys.stderr)
        return 2
    if not args.client_id:
        print("--client-id zorunlu.", file=sys.stderr)
        return 2
    if args.authorize and (not args.client_secret or not args.redirect_uri):
        print(
            "--authorize kullanırken --client-secret ve --redirect-uri zorunlu.",
            file=sys.stderr,
        )
        return 2

    ipc = DiscordIPC(raw_output=args.raw)
    try:
        pipe_path = ipc.connect()
        print(f"Discord IPC bulundu: {pipe_path}")

        ipc.send_json(
            OP_HANDSHAKE,
            {
                "v": 1,
                "client_id": str(args.client_id),
            },
        )

        opcode, ready = ipc.receive()
        if opcode != OP_FRAME or not isinstance(ready, dict) or ready.get("evt") != "READY":
            raise DiscordIPCError(f"READY bekleniyordu, gelen: {ready}")

        ready_user = (ready.get("data") or {}).get("user") or {}
        print(
            f"READY: {ready_user.get('global_name') or ready_user.get('username')} "
            f"({ready_user.get('id')})"
        )

        access_token = args.access_token
        if args.authorize:
            access_token = authorize_and_exchange(
                ipc,
                str(args.client_id),
                str(args.client_secret),
                str(args.redirect_uri),
            )

        if not access_token:
            print(
                "\nAccess token verilmedi. Şimdi GET_SELECTED_VOICE_CHANNEL "
                "gönderilecek; Discord'un kimlik doğrulamasız isteğe verdiği "
                "gerçek cevabı göreceksin."
            )
            nonce = ipc.command("GET_SELECTED_VOICE_CHANNEL", {})
            response = ipc.wait_for_nonce(nonce)
            print_packet("SONUÇ", OP_FRAME, response)
            error = rpc_error(response)
            if error and error[0] == 4006:
                print(
                    "\nBeklenen 4006 alındı: IPC bağlantısı açık, fakat ses "
                    "verisi için istemci kimlik doğrulaması gerekiyor."
                )
            return 0

        authenticate(ipc, str(access_token))
        VoiceMonitor(ipc).run()
        return 0

    except KeyboardInterrupt:
        print("\nDurduruldu.")
        return 0
    except (DiscordIPCError, requests.RequestException) as exc:
        print(f"\nHATA: {exc}", file=sys.stderr)
        return 1
    finally:
        ipc.close()


if __name__ == "__main__":
    raise SystemExit(main())
