call pnpm build
del "E:\ruanjian\沧海\canghai-api-debugger.exe"
xcopy "D:\DEV\Company\WenHui\workspace\ltsh\canghai-api\client\apps\desktop\target\release\canghai-api-debugger.exe" "E:\ruanjian\沧海" /E /I /Q