import { test, expect } from '@playwright/test';

test.describe('订单平台来源显示和筛选测试', () => {
  
  test.beforeEach(async ({ page }) => {
    // 登录
    await page.goto('https://erp.cicishop.cc/login');
    await page.fill('#username', 'admin');
    await page.fill('#password', 'admin123');
    await page.click('button[type="submit"]');
    
    // 等待登录成功跳转
    await page.waitForURL('**/dashboard**', { timeout: 5000 });
  });

  test('1. 检查订单列表是否显示平台来源列', async ({ page }) => {
    await page.goto('https://erp.cicishop.cc/orders');
    
    // 等待订单列表加载
    await page.waitForSelector('table tbody tr', { timeout: 5000 });
    
    // 检查表头是否有"平台"或"来源"列
    const tableHeaders = await page.$$eval('table thead th', ths => 
      ths.map(th => th.textContent?.trim())
    );
    
    console.log('表头列表:', tableHeaders);
    
    // 检查是否包含平台相关列
    const hasPlatformColumn = tableHeaders.some(header => 
      header?.includes('平台') || header?.includes('来源') || header?.includes('Platform')
    );
    
    expect(hasPlatformColumn).toBeTruthy();
    
    // 检查订单数据中是否显示平台信息
    const firstRow = await page.$('table tbody tr:first-child');
    if (firstRow) {
      const cells = await firstRow.$$eval('td', tds => 
        tds.map(td => td.textContent?.trim())
      );
      console.log('第一行订单数据:', cells);
    }
  });

  test('2. 检查平台筛选功能', async ({ page }) => {
    await page.goto('https://erp.cicishop.cc/orders');
    await page.waitForSelector('table tbody tr', { timeout: 5000 });
    
    // 查找平台筛选下拉框或按钮
    const platformFilter = await page.$('select[name="platform"], #platform-filter, button:has-text("平台")');
    
    if (platformFilter) {
      console.log('✅ 找到平台筛选元素');
      
      // 获取所有可用的平台选项
      const options = await platformFilter.$$eval('option', opts => 
        opts.map(opt => ({ value: opt.value, text: opt.textContent }))
      );
      console.log('平台选项:', options);
      
      // 选择特定平台（如 AliExpress）
      await platformFilter.selectOption('ali');
      
      // 等待表格刷新
      await page.waitForTimeout(1000);
      
      // 检查筛选后的订单是否都来自该平台
      const rows = await page.$$eval('table tbody tr', rows => 
        rows.map(row => {
          const cells = row.querySelectorAll('td');
          return Array.from(cells).map(cell => cell.textContent?.trim());
        })
      );
      
      console.log('筛选后的订单数据:', rows.slice(0, 3));
    } else {
      console.log('❌ 未找到平台筛选元素');
      
      // 检查是否有其他形式的筛选
      const filterSection = await page.$('text=筛选');
      if (filterSection) {
        const filterText = await filterSection.textContent();
        console.log('筛选区域文本:', filterText);
      }
    }
  });

  test('3. API 层面的平台数据验证', async ({ page }) => {
    // 通过 API 直接获取订单数据
    const response = await page.request.get('https://erp.cicishop.cc/api/orders');
    const orders = await response.json();
    
    console.log('API 返回的订单总数:', orders.length);
    
    // 统计各平台的订单数
    const platformCount: { [key: string]: number } = {};
    orders.forEach((order: any) => {
      const platform = order.platform || 'unknown';
      platformCount[platform] = (platformCount[platform] || 0) + 1;
    });
    
    console.log('API 返回的平台统计:', platformCount);
    
    // 检查是否有 platform 字段
    if (orders.length > 0) {
      const hasPlatformField = 'platform' in orders[0];
      console.log('订单对象是否有 platform 字段:', hasPlatformField);
      
      if (hasPlatformField) {
        console.log('第一个订单的平台字段:', orders[0].platform);
      }
    }
  });
});
