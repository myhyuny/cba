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

- [Chocolatey](https://chocolatey.org)
```shell
choco install 7zip.install
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

### Finder

cba를 빌드해서 원하는 경로에 설치하고 automator 디렉토리의 안의 "Compress CBA.workflow"를 Automator 로 불러와 Run Shell Script 를 cba 설치한 경로에 맞게 수정해서 설치후 사용.
