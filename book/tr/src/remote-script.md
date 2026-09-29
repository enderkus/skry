# Sunucularınızda çalışan betik

Bu sayfa skry'nin gönderdiği betiği bölüm bölüm açıklar; böylece
sunucularınıza tam olarak neyin dokunduğunu bilirsiniz. Gerçek hâlini
`skry script --all` ile yazdırabilirsiniz.

## Betik nasıl gönderilir

skry SSH üzerinden tek bir komut gönderir:

```sh
sh -c '...betik...'
```

Her şeyi `sh -c` içine sarmak, giriş kabuğunuz bash, zsh ya da fish olsa bile
betiğin POSIX kabuğunda (`/bin/sh`) çalışmasını sağlar. Betik yalnızca POSIX
özelliklerini kullanır; bu yüzden `dash` (Debian, Ubuntu), `bash` (RHEL,
Rocky) ve BusyBox `ash` (Alpine) ile çalışır.

## Giriş kısmı

```sh
LC_ALL=C; export LC_ALL
PATH="$PATH:/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin"; export PATH
T=''; if command -v timeout >/dev/null 2>&1; then T='timeout 10'; fi
m() { printf '\n@@SKRY-3f9c1a2b7d4e8f60:%s@@\n' "$1"; }
```

- `LC_ALL=C` her aracın düz İngilizce ve ondalık ayırıcı olarak `.` ile
  yazmasını sağlar; çıktı her zaman ayrıştırılabilir olur.
- `PATH` genişletilir, çünkü etkileşimsiz SSH oturumlarında `PATH` çoğu zaman
  çok kısadır ve `/usr/sbin` (`ss`'in bulunduğu yer) eksiktir.
- `timeout` aracı varsa `T`, `timeout 10` olur. Takılabilecek komutların,
  örneğin ölü bir ağ bağlamasındaki `df`'in önüne konur.
- `m`, bölümler arasına bir **işaret satırı** yazar. Uzun onaltılık kısım
  **her** ölçümde yeniden seçilen rastgele bir sayıdır. Kimse tahmin
  edemeyeceği için sunucunuzdaki hiçbir süreç adı, log satırı ya da konteyner
  adı sahte bir bölüm sınırı üretip skry'nin kafasını karıştıramaz.

## Bölümler

Her bölüm bir işaretle başlar, sonra komutlarını çalıştırır. Her komut
hatalarını `/dev/null`'a gönderir: bir araç eksikse ya da bir dosya
okunamıyorsa bölüm boş kalır ve skry `n/a` gösterir.

| Bölüm | Komut(lar) | skry ne için kullanır |
| --- | --- | --- |
| `meta` | `hostname`, `uname -r`, `uname -m`, `date`, `getconf PAGESIZE`, `getconf CLK_TCK`, `id -u` | Sunucu adı, çekirdek, mimari, saat, hesaplamalar için gereken birimler |
| `stat` | `grep -E '^(cpu\|btime\|procs_)' /proc/stat` | Toplam ve çekirdek başına CPU zaman sayaçları |
| `meminfo` | `cat /proc/meminfo` | Bellek ve swap |
| `loadavg` | `cat /proc/loadavg` | Yük ortalamaları |
| `uptime` | `cat /proc/uptime` | Çalışma süresi ve iki ölçüm arasındaki kesin süre |
| `netdev` | `cat /proc/net/dev` | Ağ arayüzü başına bayt, paket, hata |
| `diskstats` | `cat /proc/diskstats` | Blok aygıtı başına okuma, yazma ve meşgul süre |
| `df` | `df -P -k -l` (olmazsa `df -P -k`) | Dosya sistemi kullanımı |
| `procstat` | `cat /proc/[0-9]*/stat \| awk '…'` | Her sürecin CPU zamanı ve belleği (süreç başına yalnızca 6 alan tutulur) |
| `ps` | `ps -A -o pid= -o user=` (BusyBox için `ps -o pid,user`) | Her sürecin hangi kullanıcıya ait olduğu |
| `os` (yavaş) | `cat /etc/os-release` | Dağıtım adı ve sürümü |
| `cpuinfo` (yavaş) | `/proc/cpuinfo` içindeki ilk model satırı | CPU modeli |
| `ipaddr` (yavaş) | `ip -o addr show` | IP adresleri |
| `ports` (yavaş) | `ss -tlnH`, olmazsa `ss -tln`, olmazsa `netstat -tln`, olmazsa `/proc/net/tcp` | Dinleyen TCP portları |
| `containers` (yavaş) | `docker`/`podman` `ps -a` ve `stats --no-stream` | Konteynerler, durumları, CPU ve bellekleri |
| `units` (yavaş) | `systemctl --failed --plain --no-legend` (yalnızca systemd çalışıyorsa) | Çökmüş servisler |
| `logins` (yavaş) | `journalctl --since=-24h -t sshd -t sshd-session` ya da `auth.log` / `secure` / `messages` | Başarısız SSH girişleri; sunucuda `awk` ile özetlenir, geriye yalnızca birkaç satır döner |
| `updates` (yavaş) | `apt-get -s dist-upgrade`, ya da `apk version -l '<'`, ya da (isteğe bağlı) `dnf -C updateinfo list` | **Önbellekteki** paket listelerinden bekleyen güncellemeler |

## Bilmeye değer birkaç ayrıntı

- **`apt-get -s`** *simülasyon* demektir. Diskte zaten bulunan paket
  listelerini okur ve bir yükseltmenin *ne yapacağını* yazdırır. Hiçbir şey
  indirmez, kilitlemez, değiştirmez ve normal kullanıcıyla çalışır.
- **Güncellemeler, sunucunun kendi paket önbelleği kadar tazedir.** skry asla
  `apt update` çalıştırmaz. Sunucunun önbelleği bir aylıksa, bekleyen
  güncelleme listesi de bir aylıktır. Çoğu dağıtım önbelleği kendiliğinden
  yeniler (örneğin Debian/Ubuntu'da `apt-daily.timer`).
- **dnf varsayılan olarak kapalıdır.** Yalnızca önbellekten okurken bile `dnf`,
  `/var/log` altındaki kendi log dosyalarına yazar; bu, skry'nin "hiçbir şey
  yazmama" sözüyle çelişir ve root gerektirir. Bunu kabul ediyorsanız
  `security.rpm_updates = true` ile açabilirsiniz.
- **Başarısız girişler sunucuda özetlenir.** Kaba kuvvet saldırısı altında
  bir auth log yüz binlerce satır olabilir. Betik bunları kaynak adres ve saat
  başına `awk` ile sayar; geriye yalnızca bu özet (en fazla birkaç bin kısa
  satır) gönderilir.
- **`systemctl`'in çıkış kodu kontrol edilir.** D-Bus erişimi olmayan yetkisiz
  bir kullanıcıda `systemctl --failed` hiçbir şey yazmadan başarısız olur. skry
  bunu fark eder ve yanlışlıkla "çökmüş birim yok" demek yerine
  "n/a (systemctl not permitted)" gösterir.
- **Bitiş işareti.** Betiğin son satırı bir `end` işareti yazar. Bu işaret
  yoksa (örneğin komut zaman aşımına uğradıysa) skry çıktının yarıda kesildiğini
  anlar ve yine de gelen bölümleri kullanır.

## Her ölçümün parçası olmayan sorgular

Bazı komutlar kendi küçük betiklerini gönderir:

| Komut | Bölümler |
| --- | --- |
| `skry find port 5432 …` | `meta`, `ports` |
| `skry find proc nginx …` | `meta`, `procstat` ve `ps -A -o pid= -o user= -o args=` (komut satırları) |
| `skry find service nginx …` | `meta` ve `systemctl show -p Id -p LoadState -p ActiveState -p SubState -p UnitFileState -p Description -- 'nginx.service'` |

Servis adı, uzak kabuğa ulaşan **tek** kullanıcı girdisidir ve önce
denetlenir: yalnızca harfler, rakamlar ve `@ . _ : -` kabul edilir.
`find port` ve `find proc` eşleştirmeyi sizin bilgisayarınızda yapar.
