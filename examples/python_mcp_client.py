#!/usr/bin/env python3
"""
examples/python_mcp_client.py

A small, well-documented example Python client that talks to the MCP (Model Context Protocol)
server implemented in GnawTreeWriter.

Features:
- JSON-RPC 2.0 payload builder
- `initialize`, `tools/list` and `tools/call` helper methods
- Simple retry logic to wait for server readiness
- CLI for quick manual testing:
  - list:      list available tools
  - analyze:   call the 'analyze' tool for a given file path
  - call:      generic tools/call with JSON string for params

Usage examples:
  # List tools (with token)
  python examples/python_mcp_client.py --url http://127.0.0.1:8080 --token secret list

  # Analyze a file (server must be able to access the given path)
  python examples/python_mcp_client.py --url http://127.0.0.1:8080 --token secret analyze /path/to/file.py

Notes:
- The MCP endpoint is POST / and expects JSON-RPC 2.0.
- If a server token is configured, include `Authorization: Bearer <token>`.
- Tool-level errors in this MVP are represented inside `result` with `isError: true`.
  The script demonstrates how to detect both JSON-RPC errors and tool-level errors.
"""

from __future__ import annotations

import argparse
import json
import logging
import sys
import time
from typing import Any, Dict, Optional

import requests

# Configure a sensible default logger
logging.basicConfig(level=logging.INFO, format="%(levelname)s: %(message)s")
logger = logging.getLogger("mcp_client")


def build_rpc_payload(
    method: str, params: Optional[Dict[str, Any]] = None, id: int = 1
) -> Dict[str, Any]:
    """Build a JSON-RPC 2.0 payload."""
    payload = {"jsonrpc": "2.0", "method": method, "id": id, "params": params or {}}
    return payload


def post_rpc(
    url: str, payload: Dict[str, Any], token: Optional[str] = None, timeout: int = 10
) -> Dict[str, Any]:
    """Send a JSON-RPC POST and return the parsed JSON response.

    Raises requests.HTTPError on non-200 HTTP, or ValueError on invalid JSON.
    """
    headers = {"Content-Type": "application/json"}
    if token:
        headers["Authorization"] = f"Bearer {token}"

    logger.debug("POST %s", url)
    logger.debug("Payload: %s", payload)

    resp = requests.post(url, json=payload, headers=headers, timeout=timeout)
    resp.raise_for_status()
    body = resp.json()
    logger.debug("Response: %s", body)
    return body


def check_rpc_result(body: Dict[str, Any]) -> Dict[str, Any]:
    """Check the JSON-RPC body for protocol errors and return result.

    Raises RuntimeError for protocol-level errors (body['error']) and returns
    the 'result' object otherwise. The caller should then inspect `result` for
    tool-level `isError`.
    """
    if "error" in body and body["error"] is not None:
        err = body["error"]
        code = err.get("code")
        message = err.get("message")
        raise RuntimeError(f"JSON-RPC error (code {code}): {message}")
    if "result" not in body:
        raise RuntimeError("Invalid JSON-RPC response: no 'result' field")
    return body["result"]


def wait_for_server(
    url: str, token: Optional[str] = None, attempts: int = 40, sleep_secs: float = 0.05
):
    """Wait until the server responds to `initialize` or raise TimeoutError."""
    payload = build_rpc_payload("initialize", id=1)
    for i in range(attempts):
        try:
            body = post_rpc(url, payload, token=token, timeout=1)
            # If we got a response and no protocol error, consider server ready
            if "error" not in body:
                logger.debug("Server ready (initialize succeeded).")
                return
        except requests.RequestException:
            # either connection refused or other transient network error
            time.sleep(sleep_secs)
            continue
        except ValueError:
            # invalid JSON — continue trying
            time.sleep(sleep_secs)
            continue
    raise TimeoutError(f"Server at {url} did not become ready in time.")


def rpc_initialize(url: str, token: Optional[str] = None) -> Dict[str, Any]:
    body = post_rpc(url, build_rpc_payload("initialize", id=1), token=token)
    return check_rpc_result(body)


def rpc_tools_list(url: str, token: Optional[str] = None) -> Dict[str, Any]:
    body = post_rpc(url, build_rpc_payload("tools/list", id=2), token=token)
    return check_rpc_result(body)


def rpc_tools_call(
    url: str,
    name: str,
    arguments: Optional[Dict[str, Any]] = None,
    token: Optional[str] = None,
    id: int = 3,
) -> Dict[str, Any]:
    params = {"name": name}
    if arguments:
        params["arguments"] = arguments
    body = post_rpc(
        url, build_rpc_payload("tools/call", params=params, id=id), token=token
    )
    return check_rpc_result(body)


def pretty_print_result(result: Dict[str, Any]) -> None:
    """Prints a tool `result` in both human-friendly and structured forms."""
    # Tool-level error encoded in result
    if result.get("isError"):
        logger.error("Tool reported an error.")
    # human readable
    content = result.get("content")
    if content:
        for part in content:
            if isinstance(part, dict):
                text = part.get("text") or part.get("type")
                print(text)
            else:
                print(part)
    # structured content (machine readable)
    if "structuredContent" in result:
        print("\nStructured content:")
        print(json.dumps(result["structuredContent"], indent=2))


def cli_list(args: argparse.Namespace):
    logger.info("Checking server readiness...")
    wait_for_server(args.url, token=args.token)
    logger.info("Listing tools...")
    res = rpc_tools_list(args.url, token=args.token)
    tools = res.get("tools", [])
    if not tools:
        print("No tools available.")
        return
    print("Available tools:")
    for t in tools:
        name = t.get("name", "<unknown>")
        title = t.get("title", "")
        desc = t.get("description", "")
        print(f" - {name}: {title}")
        if desc:
            print(f"    {desc}")


def cli_analyze(args: argparse.Namespace):
    # The server expects a file path that it can read locally.
    file_path = args.file
    logger.info("Checking server readiness...")
    wait_for_server(args.url, token=args.token)
    logger.info("Calling analyze on %s", file_path)
    res = rpc_tools_call(
        args.url, "analyze", {"file_path": file_path}, token=args.token
    )
    pretty_print_result(res)


def cli_call(args: argparse.Namespace):
    try:
        params = json.loads(args.params) if args.params else {}
    except json.JSONDecodeError as e:
        logger.error("Invalid JSON params: %s", e)
        sys.exit(2)
    logger.info("Checking server readiness...")
    wait_for_server(args.url, token=args.token)
    logger.info("Calling tool %s ...", args.name)
    res = rpc_tools_call(args.url, args.name, params, token=args.token)
    pretty_print_result(res)


def build_arg_parser() -> argparse.ArgumentParser:
    p = argparse.ArgumentParser(
        description="Minimal MCP example client for GnawTreeWriter"
    )
    p.add_argument(
        "--url",
        default="http://127.0.0.1:8080/",
        help="MCP server URL (default: %(default)s)",
    )
    p.add_argument(
        "--token", help="Bearer token for auth (or set MCP_TOKEN in environment)"
    )

    sub = p.add_subparsers(dest="command", required=True)

    sub.add_parser("init", help="Call initialize (handshake)")

    sub_list = sub.add_parser("list", help="List available tools")

    sub_analyze = sub.add_parser("analyze", help="Call built-in 'analyze' tool")
    sub_analyze.add_argument(
        "file", help="Path to file to analyze (server must be able to access this path)"
    )

    sub_call = sub.add_parser("call", help="Call an arbitrary tool by name")
    sub_call.add_argument("name", help="Tool name to call (e.g. analyze)")
    sub_call.add_argument(
        "--params",
        help='JSON object for params.arguments (e.g. \'{"file_path":"a.py"}\')',
    )

    return p


def main(argv=None):
    parser = build_arg_parser()
    args = parser.parse_args(argv)

    # Detect token via env var if not provided explicitly
    if args.token is None:
        import os

        args.token = os.getenv("MCP_TOKEN")

    try:
        if args.command == "init":
            wait_for_server(args.url, token=args.token)
            res = rpc_initialize(args.url, token=args.token)
            logger.info("Initialize result: %s", json.dumps(res, indent=2))
        elif args.command == "list":
            cli_list(args)
        elif args.command == "analyze":
            cli_analyze(args)
        elif args.command == "call":
            cli_call(args)
        else:
            parser.print_help()
    except TimeoutError as e:
        logger.error("Timeout waiting for server: %s", e)
        sys.exit(2)
    except requests.HTTPError as e:
        logger.error("HTTP transport error: %s", e)
        sys.exit(3)
    except Exception as e:
        logger.error("Error: %s", e)
        sys.exit(4)


if __name__ == "__main__":
    main()
