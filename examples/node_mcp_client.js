#!/usr/bin/env node
/**
 * examples/node_mcp_client.js
 *
 * Minimal Node.js example client for the MCP server (JSON-RPC 2.0).
 *
 * Requirements:
 *  - Node 18+ (preferred, has native fetch)
 *  - or install node-fetch (`npm install node-fetch`) for older Node versions
 *
 * Usage:
 *  - List tools:
 *      node examples/node_mcp_client.js --url http://127.0.0.1:8080/ --token secret list
 *
 *  - Initialize (handshake):
 *      node examples/node_mcp_client.js --url http://127.0.0.1:8080/ --token secret init
 *
 *  - Analyze a file:
 *      node examples/node_mcp_client.js --url http://127.0.0.1:8080/ --token secret analyze /path/to/file.py
 *
 *  - Generic call:
 *      node examples/node_mcp_client.js --url http://127.0.0.1:8080/ --token secret call analyze '{"file_path":"examples/foo.py"}'
 *
 * Notes:
 *  - If --token is omitted, the script looks at the MCP_TOKEN environment variable.
 *  - The server must be able to access the provided file paths (they are read from the server side).
 */

'use strict';

const { argv, exit } = require('process');
const fs = require('fs');

async function getFetch() {
  if (typeof globalThis.fetch === 'function') {
    return globalThis.fetch.bind(globalThis);
  }
  // Try require (node-fetch v2) for CommonJS
  try {
    // eslint-disable-next-line global-require
    const nf = require('node-fetch');
    return nf;
  } catch (e) {
    // Try dynamic import (node-fetch v3 ESM)
    try {
      // dynamic import returns a module
      // Note: this works only if node supports dynamic import in CommonJS context
      // and 'node-fetch' is installed as an ESM module. If not installed, this will fail.
      // We'll catch and report a friendly error below.
      // eslint-disable-next-line no-undef
      const mod = await import('node-fetch');
      return mod.default;
    } catch (err) {
      console.error('This script requires fetch (Node 18+) or the `node-fetch` package.');
      console.error('To install node-fetch: npm install node-fetch');
      process.exit(1);
    }
  }
}

// Basic argument parsing (no dependency)
function parseArgs() {
  const args = argv.slice(2);
  const out = {
    url: 'http://127.0.0.1:8080/',
    token: process.env.MCP_TOKEN || null,
    cmd: null,
    cmdArgs: [],
  };

  let i = 0;
  while (i < args.length) {
    const a = args[i];
    switch (a) {
      case '--url':
        if (i + 1 >= args.length) {
          console.error('--url requires a value');
          process.exit(2);
        }
        out.url = args[++i];
        break;
      case '--token':
        if (i + 1 >= args.length) {
          console.error('--token requires a value');
          process.exit(2);
        }
        out.token = args[++i];
        break;
      case 'init':
      case 'list':
      case 'analyze':
      case 'call':
        out.cmd = a;
        i++;
        while (i < args.length) {
          out.cmdArgs.push(args[i++]);
        }
        break;
      default:
        console.error('Unknown argument:', a);
        printHelp();
        process.exit(2);
    }
    i++;
  }
  return out;
}

function printHelp() {
  console.log(`
Usage:
  --url <url>         MCP server URL (default: http://127.0.0.1:8080/)
  --token <token>     Bearer token (or set MCP_TOKEN env)

Commands:
  init                Call initialize
  list                Call tools/list
  analyze <file>      Call tools/call -> analyze { file_path }
  call <name> [json]  Call an arbitrary tool. 'json' is a JSON object for params.arguments

Examples:
  node examples/node_mcp_client.js --url http://127.0.0.1:8080/ --token secret list
  node examples/node_mcp_client.js --url http://127.0.0.1:8080/ --token secret analyze examples/foo.py
`);
}

// Build and send JSON-RPC request
async function callRpc(fetchFn, url, token, method, params = {}, id = 1) {
  const payload = { jsonrpc: '2.0', method, id, params };
  const headers = { 'Content-Type': 'application/json' };
  if (token) headers['Authorization'] = `Bearer ${token}`;
  const resp = await fetchFn(url, {
    method: 'POST',
    headers,
    body: JSON.stringify(payload),
  });
  if (!resp.ok) {
    throw new Error(`HTTP ${resp.status} ${resp.statusText}`);
  }
  const body = await resp.json();
  return body;
}

// Wait for server readiness by calling initialize several times
async function waitForServer(fetchFn, url, token, tries = 40, delayMs = 250) {
  const initPayload = { jsonrpc: '2.0', method: 'initialize', id: 1 };
  for (let i = 0; i < tries; i++) {
    try {
      const headers = { 'Content-Type': 'application/json' };
      if (token) headers['Authorization'] = `Bearer ${token}`;
      const resp = await fetchFn(url, {
        method: 'POST',
        headers,
        body: JSON.stringify(initPayload),
      });
      if (resp.ok) {
        const parsed = await resp.json();
        if (!parsed.error) return;
      }
    } catch (e) {
      // ignore, wait and retry
    }
    await new Promise((res) => setTimeout(res, delayMs));
  }
  throw new Error('Server did not become ready in time');
}

function prettyResult(result) {
  if (!result) {
    console.log('<no result>');
    return;
  }
  if (result.content) {
    console.log('--- content ---');
    for (const part of result.content) {
      if (typeof part === 'string') console.log(part);
      else if (part && part.text) console.log(part.text);
      else console.log(JSON.stringify(part));
    }
  }
  if (result.structuredContent) {
    console.log('--- structuredContent ---');
    console.log(JSON.stringify(result.structuredContent, null, 2));
  }
}

async function main() {
  const args = parseArgs();
  const fetchFn = await getFetch();

  // If no command, print help
  if (!args.cmd) {
    printHelp();
    process.exit(0);
  }

  // For commands we wait for server first (handy)
  await waitForServer(fetchFn, args.url, args.token);

  try {
    if (args.cmd === 'init') {
      const body = await callRpc(fetchFn, args.url, args.token, 'initialize', {}, 1);
      if (body.error) {
        console.error('RPC error:', JSON.stringify(body.error, null, 2));
        process.exit(2);
      }
      console.log(JSON.stringify(body.result, null, 2));
      return;
    }

    if (args.cmd === 'list') {
      const body = await callRpc(fetchFn, args.url, args.token, 'tools/list', {}, 2);
      if (body.error) {
        console.error('RPC error:', JSON.stringify(body.error, null, 2));
        process.exit(2);
      }
      console.log(JSON.stringify(body.result, null, 2));
      return;
    }

    if (args.cmd === 'analyze') {
      if (args.cmdArgs.length !== 1) {
        console.error('analyze requires a single argument: <file_path>');
        process.exit(2);
      }
      const filePath = args.cmdArgs[0];
      const body = await callRpc(
        fetchFn,
        args.url,
        args.token,
        'tools/call',
        { name: 'analyze', arguments: { file_path: filePath } },
        3
      );
      if (body.error) {
        console.error('RPC error:', JSON.stringify(body.error, null, 2));
        process.exit(2);
      }
      const result = body.result;
      if (result && result.isError) {
        console.error('Tool error:', JSON.stringify(result, null, 2));
        process.exit(3);
      }
      prettyResult(result);
      return;
    }

    if (args.cmd === 'call') {
      if (args.cmdArgs.length < 1) {
        console.error('call requires at least 1 argument: <name> [json_args]');
        process.exit(2);
      }
      const name = args.cmdArgs[0];
      const argumentJSON = args.cmdArgs[1] ? JSON.parse(args.cmdArgs[1]) : {};
      const body = await callRpc(
        fetchFn,
        args.url,
        args.token,
        'tools/call',
        { name, arguments: argumentJSON },
        4
      );
      if (body.error) {
        console.error('RPC error:', JSON.stringify(body.error, null, 2));
        process.exit(2);
      }
      const result = body.result;
      if (result && result.isError) {
        console.error('Tool error:', JSON.stringify(result, null, 2));
        process.exit(3);
      }
      prettyResult(result);
      return;
    }

    console.error('Unknown command:', args.cmd);
    printHelp();
    process.exit(2);
  } catch (e) {
    console.error('Error:', e && e.message ? e.message : e);
    process.exit(1);
  }
}

if (require.main === module) {
  main();
}
