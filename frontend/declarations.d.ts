import type Alpine from 'alpinejs';
import type shiki from 'shiki';

declare global {
  interface Window {
    Alpine: Alpine;
    shiki: Alpine;
  }
}
