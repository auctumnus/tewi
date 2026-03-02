import type { RspackOptions } from '@rspack/core';

import { readdir, mkdir, readFile, writeFile, watch } from 'fs/promises';
import path, { join, dirname, relative, extname, basename } from 'path';
import { existsSync } from 'fs';
import { transform } from 'lightningcss';
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

async function getAllFiles(
  dir: string,
  files: string[] = [],
): Promise<string[]> {
  const entries = await readdir(dir, { withFileTypes: true });

  for (const entry of entries) {
    const fullPath = join(dir, entry.name);
    if (entry.isDirectory()) {
      await getAllFiles(fullPath, files);
    } else {
      files.push(fullPath);
    }
  }

  return files;
}

async function processCSS(inputPath: string, outputPath: string) {
  const css = await readFile(inputPath, 'utf8');
  const result = transform({
    code: Buffer.from(css),
    minify: true,
    sourceMap: false,
    filename: basename(inputPath),
  });

  await writeFile(outputPath, result.code);
  console.log(
    `Processed CSS: ${relative(process.cwd(), inputPath)} → ${relative(process.cwd(), outputPath)}`,
  );
}

async function processJS(inputPath: string, outputPath: string) {
  const code = await readFile(inputPath, 'utf8');
  const result = await swcTransform(code, {
    filename: inputPath,
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
  });

  await writeFile(outputPath, result.code);
  await writeFile(outputPath + '.map', result.map || '');
  console.log(
    `Processed JS: ${relative(process.cwd(), inputPath)} → ${relative(process.cwd(), outputPath)}`,
  );
}

async function processFile(filePath: string) {
  const relativePath = relative(srcDir, filePath);
  const ext = extname(filePath);
  const baseName = basename(filePath, ext);

  // Create output directory structure
  const outputDir = join(distDir, dirname(relativePath));
  await ensureDir(outputDir);

  try {
    if (ext === '.css') {
      const outputPath = join(outputDir, `${baseName}.css`);
      await processCSS(filePath, outputPath);
    } else if (ext === '.ts' || ext === '.js') {
      const outputPath = join(outputDir, `${baseName}.js`);
      //await processJS(filePath, outputPath);
    } else {
      console.log(`Skipping: ${relativePath} (unsupported file type)`);
    }
  } catch (err: any) {
    console.error(`Error processing ${relativePath}:`, err.message);
  }
}

async function build() {
  console.log('Starting build...');

  // Ensure dist directory exists
  await ensureDir(distDir);

  // Get all files in src
  const files = await getAllFiles(srcDir);

  for (const file of files) {
    await processFile(file);
  }

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

        // Only process if file exists (not deleted)
        if (existsSync(filePath)) {
          console.log(`\nFile changed: ${event.filename}`);
          await processFile(filePath);
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
