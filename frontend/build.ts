import type { RspackOptions } from '@rspack/core';

import { readdir, mkdir, readFile, writeFile, watch } from 'fs/promises';
import path, { join, dirname, relative, extname, basename } from 'path';
import { existsSync } from 'fs';
import { transform, bundle } from 'lightningcss';
import { transform as swcTransform } from '@swc/core';
import { Compiler, rspack, Stats } from '@rspack/core';
import { TsCheckerRspackPlugin } from 'ts-checker-rspack-plugin';
import { RsdoctorRspackPlugin } from '@rsdoctor/rspack-plugin';

const srcDir = join(import.meta.dir, 'src');
const distDir = join(import.meta.dir, 'dist');

async function ensureDir(dir: string) {
  try {
    await mkdir(dir, { recursive: true });
  } catch (err: any) {
    if (err.code !== 'EEXIST') throw err;
  }
}

async function processCSS(
  inputPath: string,
  outputPath: string,
  filename?: string,
) {
  let { code } = bundle({
    filename: join(inputPath, filename ?? 'main.css'),
    minify: true,
    sourceMap: false,
    cssModules: false,
  });

  await writeFile(join(outputPath, filename ?? 'main.css'), code);
}

async function build() {
  console.log('Starting build...');

  // Ensure dist directory exists
  await ensureDir(distDir);

  await processCSS(srcDir, distDir, 'inter.css');
  await processCSS(srcDir, distDir);

  console.log('Build complete!');
}

async function watchFiles() {
  console.log('Starting watch mode...');

  // Initial build
  await build();

  console.log(`Watching ${srcDir} for changes...`);

  try {
    const watcher = watch(srcDir, { recursive: true });

    for await (const event of watcher) {
      if (event.filename) {
        const filePath = join(srcDir, event.filename);

        const ext = extname(filePath);

        // Only process if file exists (not deleted)
        if (existsSync(filePath) && ext === '.css') {
          console.log(`\nCss File changed: ${event.filename}`);
          if (event.filename === 'inter.css') {
            console.log(event.filename);
            await processCSS(srcDir, distDir, event.filename);
          } else {
            await processCSS(srcDir, distDir);
          }
        }
      }
    }
  } catch (err: any) {
    console.error('Watch error:', err.message);
  }
}

// Check command line arguments
const args = process.argv.slice(2);
const isWatchMode = args.includes('--watch') || args.includes('-w');

const rspackConfig = {
  entry: './src/home.ts',
  target: ['web', 'es2020'],
  resolve: {
    tsConfig: path.resolve('./web.tsconfig.json'),
  },
  cache: false,
  module: {
    rules: [
      {
        test: /\.ts$/,
        exclude: [/node_modules/],
        loader: 'builtin:swc-loader',
        options: {
          sourceMaps: true,
          jsc: {
            target: 'es2020',
            parser: {
              syntax: 'typescript',
            },
          },
          module: {
            type: 'es6',
          },
          minify: true,
        },
        type: 'javascript/auto',
      },
    ],
  },
  output: {
    module: true,
    chunkFormat: 'module',
    chunkLoading: 'import',
    workerChunkLoading: 'import',
    filename: 'script.js',
    path: path.resolve('./dist/'),
  },
  experiments: {
    outputModule: true,
  },
  plugins: [
    new TsCheckerRspackPlugin(),
    process.env.RSDOCTOR &&
      new RsdoctorRspackPlugin({
        // plugin options
      }),
  ],
} satisfies RspackOptions;

const rsPackCompilerRun = (compiler: Compiler): Promise<Stats | undefined> => {
  return new Promise<Stats | undefined>((resolve, reject) => {
    compiler.run((err, result) => {
      if (err !== null) {
        console.error(err);
        reject(err);
      } else {
        resolve(result);
      }
    });
  });
};

const rsPackCompilerWatch = (
  compiler: Compiler,
): Promise<Stats | undefined> => {
  return new Promise<Stats | undefined>((resolve, reject) => {
    compiler.watch({}, (err, result) => {
      console.log('Rebuilding');
      if (err !== null) {
        console.error(err);
      } else {
        console.log(result?.toString());
      }
    });
  });
};

const compiler = rspack(rspackConfig);
if (isWatchMode) {
  console.log('watch');
  Promise.all([rsPackCompilerWatch(compiler), watchFiles()]).catch(
    console.error,
  );
} else {
  Promise.all([rsPackCompilerRun(compiler), build()]).catch(console.error);
}
