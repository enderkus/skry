# Uyumluluk

## Sizin bilgisayarınız (skry'nin çalıştığı yer)

| Platform | Durum |
| --- | --- |
| Linux x86_64 ve aarch64 | Destekleniyor. Statik binary'ler, her dağıtımda çalışır. |
| macOS Apple Silicon ve Intel | Destekleniyor. |
| Windows 10/11 x86_64 | Destekleniyor. Windows OpenSSH ajanını ya da Pageant'ı ve `%USERPROFILE%\.ssh`'yi kullanır. ARM64 Windows'ta x86_64 derlemesi emülasyonla çalışır. |

Unicode ve renk destekleyen her terminal olur; Windows'ta Windows Terminal
kullanın.

## İzlenen sunucular

Sunucular, SSH sunucusu ve POSIX `sh` bulunan Linux sistemleri olmalıdır.
Sürekli test edilenler (entegrasyon testlerinde gerçek SSH sunucuları,
ayrıştırıcı testlerinde gerçek çıktılar):

| Dağıtım | Test edilen sürüm | Notlar |
| --- | --- | --- |
| Debian | 12 | Her şey destekleniyor. |
| Ubuntu | 24.04 | Her şey destekleniyor. |
| Rocky Linux (RHEL, AlmaLinux) | 9 | Bekleyen güncellemeler isteğe bağlıdır (`rpm_updates`) ve root gerektirir. |
| Alpine | 3.20 (BusyBox) | BusyBox `ps` ve `netstat` kullanır; güncellemeler için güvenlik meta verisi yok; systemd yok. |

Bunlara dayanan diğer dağıtımlar (Linux Mint, Raspberry Pi OS, CentOS
Stream, Oracle Linux, Amazon Linux, Fedora…) genellikle çalışır, çünkü skry
çekirdeğin `/proc` dosyalarına ve çok yaygın araçlara dayanır. Eksik olan her
şey `n/a` görünür.

### Sunucunun ihtiyaçları

| Zorunlu | İsteğe bağlı (yoksa özellik `n/a`) |
| --- | --- |
| sshd, `/bin/sh` (POSIX) | `ss` ya da `netstat` (portlar; son çare `/proc/net/tcp`) |
| `cat`, `grep`, `awk`, `sed`, `head` | `ip` (adresler) |
| `df`, `ps`, `uname`, `date`, `id` | `timeout` (takılan komutlara karşı koruma) |
| okunabilir bir `/proc` | `getconf` (sayfa boyutu, saat tiki; yoksa 4096 ve 100 varsayılır) |
| | `systemctl`, `journalctl` (systemd özellikleri) |
| | `docker` ya da `podman` (konteynerler) |
| | `apt-get`, `apk` ya da `dnf` (güncellemeler) |

Bunların hepsi standart kurulumlarda vardır; minimal konteyner imajlarında
birkaçı eksik olabilir.

### Çekirdek

skry uzun zamandır var olan `/proc` dosyalarını okur; bu yüzden eski
çekirdekler genellikle çalışır. Yalnızca 3.14'ten eski çekirdekler bir yedek
yönteme ihtiyaç duyar: orada `MemAvailable` yoktur ve bellek tahmin edilir
(bkz. [hesaplamalar](calculations.md#bellek)). Sürekli test edilen çekirdekler
güncel 6.x çekirdekleridir.

### İzlenen sunucu olarak desteklenmeyenler

- Linux olmayan sistemler (BSD, macOS, Windows sunucuları): skry Linux'un
  `/proc`'unu okur.
- Kendi komut satırı olan ağ cihazları (yönlendiriciler, anahtarlar).
- Kısıtlı kabuklu ya da zorunlu komutlu hesaplar.

## Hedef olarak konteynerler ve sanal makineler

skry sshd çalıştıran bir konteyneri izleyebilir, ama o zaman birçok değer
(CPU, bellek, diskler, yük) **ana makinenin çekirdeğini** anlatır, çünkü
konteynerler çekirdeği paylaşır. Anlamlı sayılar için ana makinenin kendisini
izleyin. Sanal makineler fiziksel sunucular gibi izlenir; `steal` değerine
dikkat edin.
