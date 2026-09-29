# Kurulum

skry tek bir programdır (tek bir dosya). Onu **kendi bilgisayarınıza**
kurarsınız: dizüstü bilgisayarınıza, iş istasyonunuza ya da bağlandığınız bir
atlama sunucusuna. İzlemek istediğiniz sunuculara hiçbir şey **kurmazsınız**.

> [!NOTE]
> İzlediğiniz sunucular **Linux** olmalı ve SSH ile erişilebilir olmalıdır.
> skry'yi çalıştırdığınız bilgisayar **Linux, macOS veya Windows** olabilir.

## Hızlı yol (önerilen)

### Linux ve macOS

Bir terminal açın ve yapıştırın:

```sh
curl -fsSL https://raw.githubusercontent.com/enderkus/skry/main/install.sh | sh
```

Bu komut sırasıyla şunları yapar:

1. İşletim sisteminizi ve işlemcinizi algılar (örneğin "Apple Silicon'lu
   macOS" ya da "x86_64 Linux").
2. GitHub'daki en son skry sürümünü bulur.
3. Uygun arşivi **ve** onun SHA-256 checksum dosyasını indirir.
4. Arşivin checksum ile eşleştiğini kontrol eder. Tek bir bayt bile farklıysa
   durur ve hiçbir şey kurmaz.
5. `skry` programını, yazma izniniz varsa `/usr/local/bin`'e, yoksa ev
   dizininizdeki `~/.local/bin`'e kopyalar.
6. Kurulan sürümü yazdırır.

Dizinin `PATH`'inizde olmadığını söylerse, yazdırdığı komutu uygulayın (dizini
kabuğunuzun arama yoluna ekler), sonra yeni bir terminal açın.

### Windows

**PowerShell**'i açın (eski "Komut İstemi"ni değil) ve yapıştırın:

```powershell
irm https://raw.githubusercontent.com/enderkus/skry/main/install.ps1 | iex
```

Aynı kontrolleri yapar, `skry.exe`'yi `%LOCALAPPDATA%\Programs\skry` içine
kurar ve bu klasörü kullanıcı `PATH`'inize ekler. Yeni `PATH`'in geçerli
olması için ardından **yeni bir terminal penceresi açın**.

### Sürüm ya da klasör seçmek

İki isteğe bağlı ayar kurulumun davranışını değiştirir:

| Değişken | Anlamı | Örnek |
| --- | --- | --- |
| `SKRY_VERSION` | En son sürüm yerine bu sürümü kur | `v0.1.0` |
| `SKRY_INSTALL_DIR` | Bu klasöre kur | `/opt/bin` |

```sh
curl -fsSL https://raw.githubusercontent.com/enderkus/skry/main/install.sh | SKRY_VERSION=v0.1.0 SKRY_INSTALL_DIR=/opt/bin sh
```

```powershell
$env:SKRY_VERSION = 'v0.1.0'; irm https://raw.githubusercontent.com/enderkus/skry/main/install.ps1 | iex
```

> [!TIP]
> İnternetten gelen bir betiği doğrudan kabuğa vermek içinize sinmiyor mu?
> Doğru bir refleks. [`install.sh`](https://github.com/enderkus/skry/blob/main/install.sh)
> ya da [`install.ps1`](https://github.com/enderkus/skry/blob/main/install.ps1)
> dosyasını indirin, okuyun (kısa ve açıklamalı), sonra yerel kopyayı
> `sh install.sh` ya da `./install.ps1` ile çalıştırın.

## Elle kurulum

[Sürümler sayfasındaki](https://github.com/enderkus/skry/releases) her sürüm,
her platform için bir arşiv ve bir `.sha256` dosyası içerir:

| Bilgisayarınız | İndirilecek dosya |
| --- | --- |
| Linux, Intel/AMD 64 bit | `skry-<sürüm>-x86_64-unknown-linux-musl.tar.gz` |
| Linux, ARM 64 bit (ör. Raspberry Pi 4/5, Graviton) | `skry-<sürüm>-aarch64-unknown-linux-musl.tar.gz` |
| macOS, Apple Silicon (M1/M2/M3/M4) | `skry-<sürüm>-aarch64-apple-darwin.tar.gz` |
| macOS, Intel | `skry-<sürüm>-x86_64-apple-darwin.tar.gz` |
| Windows, 64 bit | `skry-<sürüm>-x86_64-pc-windows-msvc.zip` |

İşlemcinizden emin değil misiniz? Linux veya macOS'ta `uname -m` çalıştırın:
`x86_64` Intel/AMD, `aarch64` veya `arm64` ARM demektir.

Linux x86_64 ve v0.1.0 için örnek:

```sh
curl -LO https://github.com/enderkus/skry/releases/download/v0.1.0/skry-v0.1.0-x86_64-unknown-linux-musl.tar.gz
curl -LO https://github.com/enderkus/skry/releases/download/v0.1.0/skry-v0.1.0-x86_64-unknown-linux-musl.tar.gz.sha256
sha256sum -c skry-v0.1.0-x86_64-unknown-linux-musl.tar.gz.sha256     # "OK" yazmalı
tar xzf skry-v0.1.0-x86_64-unknown-linux-musl.tar.gz
sudo install skry-v0.1.0-x86_64-unknown-linux-musl/skry /usr/local/bin/
```

macOS'ta `sha256sum -c` yerine `shasum -a 256 -c` kullanın. Dosyayı
tarayıcıyla indirdiyseniz macOS açmayı reddedebilir ("geliştirici
doğrulanamadığı için açılamıyor"). İndirme karantinasını bir kez kaldırın:

```sh
xattr -d com.apple.quarantine /usr/local/bin/skry
```

Windows'ta zip'i açın ve `skry.exe`'yi `PATH`'inizdeki herhangi bir klasöre
taşıyın.

Linux derlemeleri **statiktir**: dağıtımınızın kütüphanelerine bağlı değildir
ve eski/yeni, glibc/musl fark etmeksizin her Linux'ta çalışır.

## Kaynaktan

Rust araç zinciriniz varsa (1.85 veya üstü):

```sh
cargo install --locked --git https://github.com/enderkus/skry
```

## Çalıştığını kontrol edin

```sh
skry --version
skry demo
```

`skry demo`, arayüzü hayali bir filoyla açar — sunucu yok, SSH yok. Yardım
için `?`, çıkmak için `q`'ya basın.

## Güncelleme

Kurulum betiğini tekrar çalıştırın. Eski programın üzerine yazar.
Yapılandırmanız ve geçmişiniz korunur.

## Kaldırma

1. Programı silin: `/usr/local/bin/skry` ya da `~/.local/bin/skry`
   (Windows: `%LOCALAPPDATA%\Programs\skry` klasörü ve isterseniz kullanıcı
   `PATH`'indeki girdisi).
2. İsterseniz yapılandırmanızı ve geçmişinizi de silin. Tam konumlar için
   [skry'nin bilgisayarınızda tuttuğu dosyalar](data.md) sayfasına bakın.

Sunucularınızdan hiçbir şey kaldırmanız gerekmez, çünkü oraya hiçbir zaman
bir şey konmadı.
