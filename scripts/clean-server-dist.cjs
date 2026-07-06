const fs = require('node:fs')
const path = require('node:path')

const repoRoot = path.resolve(__dirname, '..')
const serverRoot = path.join(repoRoot, 'server')
const distDir = path.join(serverRoot, 'dist')

fs.rmSync(distDir, { recursive: true, force: true })
