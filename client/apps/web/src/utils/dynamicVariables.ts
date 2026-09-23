// Postman 风格动态变量解析：支持 {{$guid}}、{{$timestamp}}、{{$randomFirstName}} 等。
// 仅实现高频子集（数据来源为内置小词典，不依赖外部 faker 库）。

function randInt(min: number, max: number): number {
  return Math.floor(Math.random() * (max - min + 1)) + min
}
function pick<T>(arr: T[]): T {
  return arr[Math.floor(Math.random() * arr.length)]
}
function uuid(): string {
  const c = (globalThis as unknown as { crypto?: Crypto }).crypto
  if (c && typeof c.randomUUID === 'function') return c.randomUUID()
  return 'xxxxxxxx-xxxx-4xxx-yxxx-xxxxxxxxxxxx'.replace(/[xy]/g, (ch) => {
    const r = (Math.random() * 16) | 0
    const v = ch === 'x' ? r : (r & 0x3) | 0x8
    return v.toString(16)
  })
}
function hashColor(): string {
  return '#' + Math.floor(Math.random() * 0xffffff).toString(16).padStart(6, '0')
}

const FIRST_NAMES = ['James', 'Mary', 'John', 'Patricia', 'Robert', 'Jennifer', 'Michael', 'Linda', 'William', 'Elizabeth', 'David', 'Barbara', 'Richard', 'Susan', 'Joseph', 'Jessica', 'Thomas', 'Sarah', 'Charles', 'Karen', 'Lei', 'Mei', 'Wei', 'Fang', 'Yan', 'Jun']
const LAST_NAMES = ['Smith', 'Johnson', 'Williams', 'Brown', 'Jones', 'Garcia', 'Miller', 'Davis', 'Rodriguez', 'Martinez', 'Lee', 'Wang', 'Li', 'Zhang', 'Liu', 'Chen', 'Yang', 'Huang', 'Zhao', 'Wu']
const CITIES = ['Beijing', 'Shanghai', 'Guangzhou', 'Shenzhen', 'Hangzhou', 'Chengdu', 'Wuhan', 'Xi\'an', 'Nanjing', 'Chongqing', 'New York', 'London', 'Tokyo', 'Paris', 'Berlin', 'Singapore', 'Sydney', 'Toronto']
const COUNTRIES = ['China', 'United States', 'Japan', 'Germany', 'United Kingdom', 'France', 'Canada', 'Australia', 'Singapore', 'South Korea', 'India', 'Brazil']
const COUNTRY_CODES = ['CN', 'US', 'JP', 'DE', 'GB', 'FR', 'CA', 'AU', 'SG', 'KR', 'IN', 'BR']
const STREETS = ['Main St', 'High St', 'Park Ave', 'Oak Rd', 'Pine Ln', 'Maple Dr', 'Cedar Ct', 'Elm St', 'Sunset Blvd', 'Lake View']
const WORDS = ['lorem', 'ipsum', 'dolor', 'sit', 'amet', 'consectetur', 'adipiscing', 'elit', 'sed', 'tempor', 'magna', 'aliqua', 'enim', 'minim', 'veniam', 'quis', 'nostrud']
const DEPARTMENTS = ['Engineering', 'Sales', 'Marketing', 'Finance', 'HR', 'Operations', 'Legal', 'Support', 'Product', 'Design']
const JOB_AREAS = ['Cloud', 'Mobile', 'Platform', 'Data', 'Security', 'Infrastructure', 'Quality', 'Research']
const JOB_DESCRIPTORS = ['Lead', 'Senior', 'Junior', 'Principal', 'Staff', 'Associate', 'Chief', 'Global']
const JOB_TITLES = ['Engineer', 'Developer', 'Architect', 'Manager', 'Designer', 'Analyst', 'Specialist', 'Consultant', 'Administrator', 'Director']
const DOMAINS = ['example', 'test', 'demo', 'mail', 'api', 'dev', 'app', 'site', 'web', 'cloud']
const TLDS = ['com', 'org', 'net', 'io', 'cn', 'co', 'dev', 'app']
const PROTOCOLS = ['http', 'https', 'ftp', 'ws', 'wss']
const MIME_TYPES = ['application/json', 'text/plain', 'text/html', 'application/xml', 'image/png', 'application/pdf', 'multipart/form-data']
const FILE_EXTS = ['txt', 'json', 'csv', 'xml', 'pdf', 'png', 'jpg', 'md', 'log', 'yaml']
const LOREM_WORDS = WORDS
const CURRENCIES = ['USD', 'CNY', 'EUR', 'JPY', 'GBP', 'HKD', 'SGD', 'AUD']
const LANGUAGES = ['en', 'zh-CN', 'ja', 'de', 'fr', 'es', 'ru', 'pt', 'ko', 'ar']
const LOCALES = ['en-US', 'zh-CN', 'ja-JP', 'de-DE', 'fr-FR', 'en-GB', 'ko-KR', 'pt-BR']
const MONTHS = ['January', 'February', 'March', 'April', 'May', 'June', 'July', 'August', 'September', 'October', 'November', 'December']
const WEEKDAYS = ['Monday', 'Tuesday', 'Wednesday', 'Thursday', 'Friday', 'Saturday', 'Sunday']
const USER_AGENTS = [
  'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0 Safari/537.36',
  'Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.0 Safari/605.1.15',
  'Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/119.0 Safari/537.36',
]

function randomIp(): string {
  return `${randInt(1, 255)}.${randInt(0, 255)}.${randInt(0, 255)}.${randInt(1, 255)}`
}
function randomIpv6(): string {
  const seg = () => Math.floor(Math.random() * 0xffff).toString(16)
  return `${seg()}:${seg()}:${seg()}:${seg()}:${seg()}:${seg()}:${seg()}:${seg()}`
}
function randomEmail(): string {
  return `${pick(FIRST_NAMES).toLowerCase()}.${pick(LAST_NAMES).toLowerCase()}${randInt(1, 999)}@${pick(DOMAINS)}.${pick(TLDS)}`
}
function randomUrl(): string {
  return `${pick(PROTOCOLS)}://${pick(DOMAINS)}.${pick(TLDS)}/${pick(WORDS)}`
}
function randomUserName(): string {
  return `${pick(FIRST_NAMES).toLowerCase()}_${pick(LAST_NAMES).toLowerCase()}${randInt(1, 999)}`
}
function randomPassword(): string {
  const chars = 'abcdefghijkmnpqrstuvwxyzABCDEFGHJKLMNPQRSTUVWXYZ23456789!@#$%'
  let s = ''
  for (let i = 0; i < 12; i++) s += chars[randInt(0, chars.length - 1)]
  return s
}
function loremSentence(n = 8): string {
  const arr: string[] = []
  for (let i = 0; i < n; i++) arr.push(pick(LOREM_WORDS))
  return arr.join(' ') + '.'
}

const GENERATORS: Record<string, () => string> = {
  // 基础
  $guid: uuid,
  $randomUUID: uuid,
  $timestamp: () => String(Math.floor(Date.now() / 1000)),
  $isoTimestamp: () => new Date().toISOString(),
  $randomInt: () => String(randInt(0, 1000)),
  $randomBoolean: () => String(Math.random() < 0.5),
  $randomColor: hashColor,
  $randomAlphaNumeric: () => Math.random().toString(36).slice(2, 10),
  $randomSemver: () => `${randInt(0, 9)}.${randInt(0, 9)}.${randInt(0, 20)}`,

  // 人物
  $randomFirstName: () => pick(FIRST_NAMES),
  $randomLastName: () => pick(LAST_NAMES),
  $randomFullName: () => `${pick(FIRST_NAMES)} ${pick(LAST_NAMES)}`,
  $randomName: () => `${pick(FIRST_NAMES)} ${pick(LAST_NAMES)}`,
  $randomUserName: randomUserName,
  $randomPassword: randomPassword,
  $randomJobArea: () => pick(JOB_AREAS),
  $randomJobDescriptor: () => pick(JOB_DESCRIPTORS),
  $randomJobTitle: () => `${pick(JOB_DESCRIPTORS)} ${pick(JOB_TITLES)}`,
  $randomDepartment: () => pick(DEPARTMENTS),

  // 网络 / 互联网
  $randomEmail: randomEmail,
  $randomUrl: randomUrl,
  $randomDomainName: () => `${pick(DOMAINS)}.${pick(TLDS)}`,
  $randomIP: randomIp,
  $randomIPV6: randomIpv6,
  $randomUserAgent: () => pick(USER_AGENTS),
  $randomProtocol: () => pick(PROTOCOLS),
  $randomPort: () => String(randInt(1024, 65535)),

  // 地址
  $randomCity: () => pick(CITIES),
  $randomStreetName: () => pick(STREETS),
  $randomCountry: () => pick(COUNTRIES),
  $randomCountryCode: () => pick(COUNTRY_CODES),
  $randomState: () => pick(CITIES),
  $randomZipCode: () => String(randInt(100000, 999999)),
  $randomLatitude: () => (Math.random() * 180 - 90).toFixed(6),
  $randomLongitude: () => (Math.random() * 360 - 180).toFixed(6),

  // 商业 / 金融
  $randomPrice: () => (Math.random() * 1000).toFixed(2),
  $randomBankAccount: () => String(randInt(1000000000, 9999999999)),
  $randomBankAccountName: () => `${pick(FIRST_NAMES)} ${pick(LAST_NAMES)}`,
  $randomCreditCardMask: () => `**** **** **** ${randInt(1000, 9999)}`,
  $randomCurrencyCode: () => pick(CURRENCIES),
  $randomCurrencyName: () => ({
    USD: 'US Dollar', CNY: 'Chinese Yuan', EUR: 'Euro', JPY: 'Japanese Yen',
    GBP: 'British Pound', HKD: 'Hong Kong Dollar', SGD: 'Singapore Dollar', AUD: 'Australian Dollar',
  }[pick(CURRENCIES)] ?? 'US Dollar'),
  $randomCurrencySymbol: () => ({ USD: '$', CNY: '¥', EUR: '€', JPY: '¥', GBP: '£', HKD: 'HK$', SGD: 'S$', AUD: 'A$' }[pick(CURRENCIES)] ?? '$'),

  // 日期 / 时间
  $randomDateRecent: () => new Date(Date.now() - randInt(0, 30) * 86400000).toISOString().slice(0, 10),
  $randomDateFuture: () => new Date(Date.now() + randInt(1, 30) * 86400000).toISOString().slice(0, 10),
  $randomMonth: () => pick(MONTHS),
  $randomWeekday: () => pick(WEEKDAYS),

  // 文本
  $randomWord: () => pick(WORDS),
  $randomLoremWord: () => pick(LOREM_WORDS),
  $randomLoremSentence: () => loremSentence(),
  $randomLoremParagraph: () => `${loremSentence(10)} ${loremSentence(10)}`,

  // 文件 / 系统
  $randomFileExt: () => pick(FILE_EXTS),
  $randomFileName: () => `${pick(WORDS)}.${pick(FILE_EXTS)}`,
  $randomMimeType: () => pick(MIME_TYPES),
  $randomLocale: () => pick(LOCALES),
  $randomLanguage: () => pick(LANGUAGES),
}

/** 解析 Postman 动态变量；不支持的名称返回 null（交由上层回退为原样 {{name}}）。 */
export function resolveDynamicVariable(name: string): string | null {
  const fn = GENERATORS[name]
  if (!fn) return null
  try {
    return fn()
  } catch {
    return null
  }
}
