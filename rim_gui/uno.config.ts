// uno.config.ts
import {
  defineConfig,
  presetAttributify,
  presetIcons,
  presetTypography,
  presetUno,
  presetWebFonts,
  transformerDirectives,
  transformerVariantGroup,
} from 'unocss';

export default defineConfig({
  shortcuts: [
    // ...
  ],

  theme: {
    colors: {
      // 背景色
      'secondary-btn': '#f2f2f2',
      'disabled-bg': '#eeeeee',

      // 主色
      primary: '#5b98d8',
      success: '#50d4ab',
      warning: '#fbb175',
      danger: '#f66f6a',
      info: '#909399',
      'light-primary': '#e3f1ff',
      'deep-primary': '#40569f',

      // 文字色相关
      header: '#252b3a',
      regular: '#575d6c',
      'darker-secondary': '#6a6a6a',
      secondary: '#808080',
      placeholder: '#adb0b8',
      disabled: '#c0c4cc',
      reverse: '#ffffff',
      active: '#5e7ce0',

      // 边框色
      base: '#adb0b8',
      light: '#dfe1e6',
      lighter: '#ebeef5',
      'extra-light': '#f2f6fc',
      dark: '#d4d7de',
      darker: '#cdd0d6',
      gold: '#d5bc7B',
    },
  },
  presets: [
    presetUno(),
    presetAttributify(),
    presetIcons({
      extraProperties: { display: 'inline-block', "vertical-align": 'middle' },
    }),
    presetTypography(),
    presetWebFonts({
      fonts: {
        // ...
      },
    }),
  ],
  transformers: [
    transformerDirectives({
      applyVariable: ['--uno'],
    }),
    transformerVariantGroup(),
  ],
});
