import { invoke } from '@tauri-apps/api';
import { message } from '@tauri-apps/api/dialog';

// 使用 message invoke 显示错误信息
export async function invokeCommand(command: string, args = {}) {
  try {
    return await invoke(command, args);
  } catch (error: any) {
    // 捕获错误并显示对话框
    await message(error || '发生了一个错误', {
      title: '错误',
      type: 'error',
    });
    throw error; // 重新抛出错误以便外部的 .catch 继续处理
  }
}

/**
 * 递归解析并获取完整的 label 字符串
 * 该方法只适用于静态内容，即%{}中的变量只能来自i18n
 * @param key - 要获取的 label 的键
 * @param params - 占位符变量的初始参数
 * @returns 完整解析后的字符串
 */
export async function invokeLabel(key: string, params: Record<string, string> = {}): Promise<string> {
  const response = await invokeCommand('get_label', { key, ...params }) as string;

  // 匹配 %{} 占位符
  const placeholderRegex = /%\{(.*?)\}/g;
  const matches = [...response.matchAll(placeholderRegex)];

  if (matches.length > 0) {
    for (const match of matches) {
      const placeholderKey = match[1]; // 提取占位符内部的变量名

      // 递归获取占位符的值
      const placeholderValue = await invokeLabel(placeholderKey);
      params[placeholderKey] = placeholderValue;
    }
    return response.replace(placeholderRegex, (_, key) => params[key] ?? `%{${key}}`);
  }

  return response;
}

/**
 * 获取多个 label 字符串
 * @param labelList - 要获取的 label 的键列表
 * @returns 完整解析后的字符串
 */
export async function invokeLabelList(labelList: string[]) {
  const labelPromise = labelList.map((key) => invokeLabel(key));
  return Promise.all(labelPromise).then((results) => {
    return results.reduce(
      (acc, result, index) => {
        acc[labelList[index]] = result;
        return acc;
      },
      {} as Record<string, string>
    );
  });
}
