//@ts-check

'use strict';

const path = require('path');
const { execFileSync } = require('child_process');
const webpack = require('webpack');

// Resolve the release endpoint from an explicit build setting, falling back to the checkout's
// GitHub origin. Builds without either (source archives, unusual remotes) still work; only
// bundled downloads need the endpoint.
function resolveReleasesUrl() {
  const explicit = process.env.STYLUAU_RELEASES_URL;
  if (explicit && explicit.trim() !== '') {
    return explicit.trim();
  }

  let origin;
  try {
    origin = execFileSync('git', ['remote', 'get-url', 'origin'], {
      encoding: 'utf8',
      stdio: ['ignore', 'pipe', 'ignore'],
    }).trim();
  } catch {
    return null;
  }

  const match = origin.match(
    /^(?:https?:\/\/github\.com\/|git@github\.com:|ssh:\/\/git@github\.com\/)([^/]+)\/([^/]+?)(?:\.git)?\/?$/i
  );
  return match
    ? `https://api.github.com/repos/${match[1]}/${match[2]}/releases`
    : null;
}

const releasesUrl = resolveReleasesUrl();

//@ts-check
/** @typedef {import('webpack').Configuration} WebpackConfig **/

/** @type WebpackConfig */
const extensionConfig = {
  target: 'node', // vscode extensions run in a Node.js-context 📖 -> https://webpack.js.org/configuration/node/
	mode: 'none', // this leaves the source code as close as possible to the original (when packaging we set this to 'production')

  plugins: [new webpack.DefinePlugin({ STYLUAU_RELEASES_URL: JSON.stringify(releasesUrl) })],
  entry: './src/extension.ts', // the entry point of this extension, 📖 -> https://webpack.js.org/configuration/entry-context/
  output: {
    // the bundle is stored in the 'dist' folder (check package.json), 📖 -> https://webpack.js.org/configuration/output/
    path: path.resolve(__dirname, 'dist'),
    filename: 'extension.js',
    libraryTarget: 'commonjs2'
  },
  externals: {
    vscode: 'commonjs vscode' // the vscode-module is created on-the-fly and must be excluded. Add other modules that cannot be webpack'ed, 📖 -> https://webpack.js.org/configuration/externals/
    // modules added here also need to be added in the .vscodeignore file
  },
  resolve: {
    // support reading TypeScript and JavaScript files, 📖 -> https://github.com/TypeStrong/ts-loader
    extensions: ['.ts', '.js']
  },
  module: {
    rules: [
      {
        test: /\.ts$/,
        exclude: /node_modules/,
        use: [
          {
            loader: 'ts-loader'
          }
        ]
      }
    ]
  },
  devtool: 'nosources-source-map',
  infrastructureLogging: {
    level: "log", // enables logging required for problem matchers
  },
};
module.exports = [ extensionConfig ];