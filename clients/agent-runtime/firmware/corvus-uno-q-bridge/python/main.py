# Corvus Bridge — socket server for GPIO control from Corvus agent
# SPDX-License-Identifier: MPL-2.0

import socket
import threading
from arduino.app_utils import App, Bridge

corvus_PORT = 9999

def handle_client(conn):
    """
    Handle a single client connection, processing a simple GPIO text command and replying with the result.
    
    Reads a single whitespace-separated command from the connection and responds on the same socket. Supported commands:
    - "gpio_write <pin> <value>": calls Bridge.call("digitalWrite", [pin, value]) and sends "ok\n".
    - "gpio_read <pin>": calls Bridge.call("digitalRead", [pin]) and sends "<value>\n".
    
    If the command is malformed, sends "error: invalid command\n". If the command is unrecognized, sends "error: unknown command\n". If an exception occurs while processing, attempts to send "error: {e}\n". The connection is closed before returning.
    
    Parameters:
        conn: A socket-like object representing the client connection; must support recv, sendall, and close.
    """
    try:
        data = conn.recv(256).decode().strip()
        if not data:
            conn.close()
            return
        parts = data.split()
        if len(parts) < 2:
            conn.sendall(b"error: invalid command\n")
            conn.close()
            return
        cmd = parts[0].lower()
        if cmd == "gpio_write" and len(parts) >= 3:
            pin = int(parts[1])
            value = int(parts[2])
            Bridge.call("digitalWrite", [pin, value])
            conn.sendall(b"ok\n")
        elif cmd == "gpio_read" and len(parts) >= 2:
            pin = int(parts[1])
            val = Bridge.call("digitalRead", [pin])
            conn.sendall(f"{val}\n".encode())
        else:
            conn.sendall(b"error: unknown command\n")
    except Exception as e:
        try:
            conn.sendall(f"error: {e}\n".encode())
        except Exception:
            # Intentionally ignore failures while sending the error response
            pass
    finally:
        conn.close()

def accept_loop(server):
    while True:
        try:
            conn, _ = server.accept()
            t = threading.Thread(target=handle_client, args=(conn,))
            t.daemon = True
            t.start()
        except Exception:
            break

def loop():
    App.sleep(1)

def main():
    server = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
    server.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
    server.bind(("127.0.0.1", corvus_PORT))
    server.listen(5)
    server.settimeout(1.0)
    t = threading.Thread(target=accept_loop, args=(server,))
    t.daemon = True
    t.start()
    App.run(user_loop=loop)

if __name__ == "__main__":
    main()
