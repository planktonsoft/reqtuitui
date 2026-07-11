#!/usr/bin/env python3
import http.server
import sys
from urllib.parse import urlparse, parse_qs
import json

PORT = 8080

class RequestLoggerHandler(http.server.BaseHTTPRequestHandler):
    def log_message(self, format, *args):
        # Suppress default request logging to avoid duplication
        pass

    def handle_request(self):
        print("\n" + "=" * 60)
        print(f"📩 RECEIVED HTTP REQUEST")
        print("=" * 60)
        
        # 1. Connection / Request Details
        print(f"Client Address : {self.client_address[0]}:{self.client_address[1]}")
        print(f"Request Line   : {self.command} {self.path} {self.request_version}")
        
        # 2. Parse URL & Query Parameters
        parsed_url = urlparse(self.path)
        print(f"Path           : {parsed_url.path}")
        query_params = parse_qs(parsed_url.query)
        if query_params:
            print("Query Params   :")
            for key, val in query_params.items():
                print(f"  - {key}: {', '.join(val)}")
        else:
            print("Query Params   : None")

        print("-" * 60)

        # 3. HTTP Headers
        print("Headers        :")
        for header, val in self.headers.items():
            print(f"  {header}: {val}")

        # Highlight Cookies explicitly
        cookie = self.headers.get("Cookie")
        if cookie:
            print(f"\n🍪 Detected Cookies: {cookie}")

        print("-" * 60)

        # 4. Request Body
        content_length = int(self.headers.get('Content-Length', 0))
        if content_length > 0:
            raw_body = self.rfile.read(content_length)
            print(f"Body (Content-Length: {content_length} bytes):")
            try:
                # Try parsing and pretty-printing JSON
                decoded_body = raw_body.decode('utf-8')
                json_data = json.loads(decoded_body)
                print(json.dumps(json_data, indent=2))
            except (UnicodeDecodeError, json.JSONDecodeError, ValueError):
                try:
                    # Print raw text
                    print(raw_body.decode('utf-8'))
                except UnicodeDecodeError:
                    # Hex dump for binary data
                    print(raw_body.hex())
        else:
            print("Body           : None")

        print("=" * 60 + "\n")
        
        # Respond 200 OK
        self.send_response(200)
        self.send_header("Content-Type", "text/plain")
        self.end_headers()
        self.wfile.write(b"HTTP 200 OK - Request logged successfully!")

    def do_GET(self):
        self.handle_request()

    def do_POST(self):
        self.handle_request()

    def do_PUT(self):
        self.handle_request()

    def do_DELETE(self):
        self.handle_request()

    def do_PATCH(self):
        self.handle_request()

    def do_OPTIONS(self):
        self.handle_request()

    def do_HEAD(self):
        self.handle_request()

def run():
    server_address = ('127.0.0.1', PORT)
    httpd = http.server.HTTPServer(server_address, RequestLoggerHandler)
    print(f"🚀 Fully-detailed Mock Server running at http://127.0.0.1:{PORT}")
    print("Send requests from your TUI and monitor the request details below.")
    try:
        httpd.serve_forever()
    except KeyboardInterrupt:
        print("\n👋 Mock server stopped.")
        sys.exit(0)

if __name__ == "__main__":
    run()
