# cba

이미지 정리해서 압축 관리하기 귀찮다.

## 필수 프로그램 설치

### macOS

- [Homebrew](https://brew.sh)
```shell
brew install sevenzip
```

- [MacPorts](https://www.macports.org)
```shell
port install p7zip
```

### Windows

- Windows Package Manager
```shell
winget install 7zip.7zip
```
- Download
    * https://www.7-zip.org

혹시 위에 설치해도 안되면 7z 파일이 있는 곳을 PATH에 등록해줄것.

#### 

## 사용방법

### Terminal

```shell
cba [directory...]
```
