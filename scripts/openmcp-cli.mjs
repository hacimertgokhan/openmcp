#!/usr/bin/env node
import fs from 'node:fs'
import os from 'node:os'
import path from 'node:path'

const catalog = {
  context7: {
    type: 'remote',
    url: 'https://mcp.context7.com/mcp',
    enabled: true,
  },
  playwright: {
    type: 'local',
    command: ['npx', '-y', '@playwright/mcp@latest'],
    enabled: true,
  },
  filesystem: {
    type: 'local',
    command: ['npx', '-y', '@modelcontextprotocol/server-filesystem', '{PROJECT_PATH}'],
    enabled: true,
  },
  github: {
    type: 'local',
    command: ['npx', '-y', '@modelcontextprotocol/server-github'],
    environment: {
      GITHUB_PERSONAL_ACCESS_TOKEN: '{env:GITHUB_PERSONAL_ACCESS_TOKEN}',
    },
    enabled: true,
  },
  postgres: {
    type: 'local',
    command: ['npx', '-y', '@modelcontextprotocol/server-postgres', '{env:POSTGRES_CONNECTION_STRING}'],
    enabled: true,
  },
  'mysql-readonly': {
    type: 'local',
    command: ['npx', '-y', 'mysql-readonly-mcp'],
    environment: {
      MYSQL_HOST: '{env:MYSQL_HOST}',
      MYSQL_PORT: '{env:MYSQL_PORT}',
      MYSQL_USER: '{env:MYSQL_USER}',
      MYSQL_PASSWORD: '{env:MYSQL_PASSWORD}',
      MYSQL_DATABASE: '{env:MYSQL_DATABASE}',
    },
    enabled: true,
  },
}

function usage() {
  console.log(`OpenMCP CLI

Usage:
  openmcp list [--config path]
  openmcp enable <name> [--config path]
  openmcp disable <name> [--config path]
  openmcp remove <name> [--config path]
  openmcp install <catalog-name> [--as name] [--config path]
  openmcp project-add <name> --project path [--config path]
  openmcp catalog

Every write creates a .bak-<timestamp> backup.`)
}

const args = process.argv.slice(2)
const command = args[0]

if (!command || command === '-h' || command === '--help') {
  usage()
  process.exit(0)
}

function option(name) {
  const index = args.indexOf(name)
  return index >= 0 ? args[index + 1] : undefined
}

function configPath() {
  return path.resolve(option('--config') ?? process.env.OPENCODE_CONFIG ?? path.join(os.homedir(), '.config', 'opencode', 'opencode.json'))
}

function stripJsonComments(input) {
  let output = ''
  let inString = false
  let escaped = false
  for (let index = 0; index < input.length; index += 1) {
    const char = input[index]
    const next = input[index + 1]
    if (inString) {
      output += char
      if (escaped) escaped = false
      else if (char === '\\') escaped = true
      else if (char === '"') inString = false
      continue
    }
    if (char === '"') {
      inString = true
      output += char
      continue
    }
    if (char === '/' && next === '/') {
      while (index < input.length && input[index] !== '\n') index += 1
      output += '\n'
      continue
    }
    if (char === '/' && next === '*') {
      index += 2
      while (index < input.length && !(input[index] === '*' && input[index + 1] === '/')) index += 1
      index += 1
      continue
    }
    output += char
  }
  return output
}

function removeTrailingCommas(input) {
  let output = ''
  let inString = false
  let escaped = false
  for (let index = 0; index < input.length; index += 1) {
    const char = input[index]
    if (inString) {
      output += char
      if (escaped) escaped = false
      else if (char === '\\') escaped = true
      else if (char === '"') inString = false
      continue
    }
    if (char === '"') {
      inString = true
      output += char
      continue
    }
    if (char === ',') {
      let lookahead = index + 1
      while (lookahead < input.length && /\s/.test(input[lookahead])) lookahead += 1
      if (input[lookahead] === '}' || input[lookahead] === ']') continue
    }
    output += char
  }
  return output
}

function normalizeJsonc(input) {
  return removeTrailingCommas(stripJsonComments(input))
}

function readConfig(file) {
  if (!fs.existsSync(file)) return {}
  return JSON.parse(normalizeJsonc(fs.readFileSync(file, 'utf8')))
}

function redactSensitiveText(value = '') {
  return String(value)
    .replace(/:\/\/([^/@?#]+)@/g, '://<redacted>@')
    .replace(/([?&][^=]*(?:password|passwd|pwd|token|secret|key|auth|credential)[^=]*=)[^&#\s]+/gi, '$1<redacted>')
    .replace(/((?:password|passwd|pwd|token|secret|api_key|apikey|access_key)\s*[=:]\s*)[^&;,\s]+/gi, '$1<redacted>')
}

function writeConfig(file, data) {
  fs.mkdirSync(path.dirname(file), { recursive: true })
  if (fs.existsSync(file)) {
    fs.copyFileSync(file, `${file}.bak-${Math.floor(Date.now() / 1000)}`)
  }
  fs.writeFileSync(file, `${JSON.stringify(data, null, 2)}\n`)
}

function ensureMcp(config) {
  if (!config.mcp || typeof config.mcp !== 'object' || Array.isArray(config.mcp)) config.mcp = {}
  return config.mcp
}

function setEnabled(name, enabled) {
  const file = configPath()
  const config = readConfig(file)
  const mcp = ensureMcp(config)
  if (!mcp[name]) throw new Error(`MCP not found: ${name}`)
  mcp[name].enabled = enabled
  writeConfig(file, config)
  console.log(`${name}: ${enabled ? 'enabled' : 'disabled'}`)
}

function remove(name) {
  const file = configPath()
  const config = readConfig(file)
  const mcp = ensureMcp(config)
  if (!mcp[name]) throw new Error(`MCP not found: ${name}`)
  delete mcp[name]
  writeConfig(file, config)
  console.log(`${name}: removed`)
}

function install(name) {
  const item = catalog[name]
  if (!item) throw new Error(`Catalog item not found: ${name}`)
  const asName = option('--as') ?? name
  const file = configPath()
  const config = readConfig(file)
  ensureMcp(config)[asName] = { ...item, enabled: item.enabled ?? true }
  writeConfig(file, config)
  console.log(`${asName}: installed to ${file}`)
}

function list() {
  const file = configPath()
  const config = readConfig(file)
  const entries = Object.entries(config.mcp ?? {})
  if (!entries.length) {
    console.log('No MCP servers found.')
    return
  }
  for (const [name, definition] of entries) {
    const type = definition.type ?? 'local'
    const enabled = definition.enabled === false ? 'disabled' : 'enabled'
    const target = redactSensitiveText(definition.url ?? definition.command?.map(redactSensitiveText).join(' ') ?? '')
    console.log(`${enabled.padEnd(8)} ${type.padEnd(6)} ${name} ${target}`)
  }
}

function projectAdd(name) {
  const project = option('--project')
  if (!project) throw new Error('--project is required')
  const source = readConfig(configPath())
  const definition = source.mcp?.[name]
  if (!definition) throw new Error(`MCP not found: ${name}`)
  const target = path.join(path.resolve(project), 'opencode.json')
  const targetConfig = readConfig(target)
  ensureMcp(targetConfig)[name] = definition
  writeConfig(target, targetConfig)
  console.log(`${name}: added to ${target}`)
}

try {
  if (command === 'list') list()
  else if (command === 'enable') setEnabled(args[1], true)
  else if (command === 'disable') setEnabled(args[1], false)
  else if (command === 'remove') remove(args[1])
  else if (command === 'install') install(args[1])
  else if (command === 'project-add') projectAdd(args[1])
  else if (command === 'catalog') console.log(Object.keys(catalog).join('\n'))
  else {
    usage()
    process.exitCode = 1
  }
} catch (error) {
  console.error(error instanceof Error ? error.message : error)
  process.exitCode = 1
}
