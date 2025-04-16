# Graphical User Interface for `RIM`

This is the front-end for `RIM` using `Tauri` + `Vue` + `Typescript`.

## 推荐使用IDE和插件

- `VS Code` + `Vue-official` + `Tauri` + `rust-analyzer` + `Prettier-Code formatter` + `UnoCSS/UnoT`

## 开始

**推荐使用pnpm对前端依赖进行管理**
```
npm install pnpm -g
```

**下载前端依赖**
```
pnpm install
```

**运行**
```
pnpm run tauri dev
```

**构建**
```
pnpm run tauri build
```

## 说明
- Vue 和 TypeScript文件使用Prettier进行代码格式化，但依赖中没有包含prettier，可以添加全局包
  ```
  npm install prettier -g
  ```
  或者
  ```
  pnpm add prettier -g
  ```
  嫌麻烦也可以添加到项目依赖中
  ```
  cd ./rim_gui
  pnpm add --save-dev --save-exact prettier
  ```

- `tauri` API 可以从依赖 `@tauri-apps/api` 中引入
  ```ts
  import { invoke } from '@tauri-apps/api/core';
  import { event } from '@tauri-apps/api';
  ```

- 页面代码在文件夹 `rim_gui/src/view` 中
  ```
  --install
    --src
      --view
        --HomeView.vue
        ...
  ```
