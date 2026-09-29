# Sorun giderme

Buradan başlayın: **düz SSH çalışıyor mu?**

```sh
ssh -o BatchMode=yes web1 true && echo calisiyor
```

`BatchMode=yes`, `ssh`'in skry gibi davranmasını sağlar (parola sormaz). Bu
başarısızsa önce SSH'yi düzeltin; skry, `ssh`'ten daha iyisini yapamaz. Sonra
aşağıda belirtinizi bulun.

skry'nin ne yaptığını görmek için bir log dosyasıyla çalıştırın:

```sh
skry -vv --log-file /tmp/skry.log web1
```

## Bağlanma

**"unknown host key for …"**
Sunucu `known_hosts`'ta yok. Parmak izini doğrulayıp bir kez `--accept-new`
ile çalıştırın ya da bir kez `ssh` ile bağlanın. → [Host anahtarları](host-keys.md)

**"HOST KEY MISMATCH for …"**
Sunucunun anahtarı değişti. Bir şey yapmadan önce nedenini öğrenin (yeniden
kurulum? yeniden kullanılan IP? saldırı?). Sonra `ssh-keygen -R <sunucu>` ve
yeniden bağlanın.
→ [Host anahtarları](host-keys.md#host-key-mismatch--şimdi-ne-olacak)

**"authentication failed for user@host (tried N keys)"**
- Kullanıcı doğru mu? `ssh`'in kullandığı kullanıcıyla karşılaştırın
  (`ssh -G web1 | grep '^user '`).
- Açık anahtarınız sunucuda o kullanıcının `~/.ssh/authorized_keys`
  dosyasında mı?
- Anahtar yüklü mü? `ssh-add -l` ajandaki anahtarları listeler.
- Anahtar parola korumalı ve skry terminalsiz mi çalışıyor (cron, CI)? Onu bir
  ajana yükleyin.
→ [Kimlik doğrulama](authentication.md)

**"… server closed the connection after N keys"**
Ajanınız çok fazla anahtar sundu ve sunucunun `MaxAuthTries` hakkı bitti. O
sunucu için `IdentityFile` ve `IdentitiesOnly yes` ayarlayın.

**"no usable keys: start ssh-agent or set IdentityFile"**
skry hiç anahtar bulamadı. `~/.ssh/id_ed25519`'un (ya da yapılandırdığınız
anahtarın) var olduğunu kontrol edin ya da bir ajan başlatıp `ssh-add` yapın.

**"timed out while connecting"**
Bir güvenlik duvarı trafiği sessizce düşürüyor, adres yanlış ya da sunucu
kapalı. `nc -vz <sunucu> 22` deneyin. Yavaş bir VPN arkasındaysanız
`connect_timeout`'u artırın.

**"cannot connect … Connection refused"**
O portta dinleyen yok. sshd çalışıyor mu? Port doğru mu?

**"cannot resolve …"**
İsim ne `~/.ssh/config`'te ne DNS'te var. SSH yapılandırmanız başka yerdeyse
`--ssh-config` kullanın.

**"via jump host bastion: …"**
Hata bastion'da oldu. Önce `ssh bastion`'ı kontrol edin.

**"remote command failed" / "produced no usable output"**
Giriş yaptınız ama betik çalışamadı: kısıtlı kabuk (`rbash`), `nologin`,
`authorized_keys`'te zorunlu bir `command=` ya da çıkış yapan bir giriş betiği.
Hesaba normal bir kabuk verin.

## Veriler

**Başlangıçtan hemen sonra CPU ve ağ `n/a` gösteriyor**
Normal: hızlar iki ölçüm gerektirir. Yaklaşık bir saniye sonra görünürler.

**Bir panel `n/a (…)` diyor**
Neden parantezin içindedir. Yaygın olanlar:

| Mesaj | Çözüm |
| --- | --- |
| `n/a (logs not readable (needs adm or systemd-journal group))` | İzleme kullanıcısını `adm`/`systemd-journal` grubuna ekleyin |
| `n/a (no journal or auth log)` | Sunucuda ne journald ne okunabilir bir auth log dosyası var |
| `n/a (systemctl not permitted)` | D-Bus erişimi olmayan root dışı kullanıcı; root kullanmıyorsanız yapılacak bir şey yok |
| `n/a (no systemd)` | Sunucu systemd kullanmıyor (ör. Alpine) |
| `n/a (docker: permission denied)` | Kullanıcının Docker soketini kullanma izni yok ([neden önemli](permissions.md)) |
| `n/a (no docker or podman)` | İkisi de kurulu değil |
| `n/a (requires root on RPM systems)` | dnf güncelleme kontrolleri root ister |
| `n/a (disabled on RPM systems; set security.rpm_updates)` | `rpm_updates = true` ile açın |
| `n/a (no supported package manager)` | apt, apk ya da dnf değil |

**Bellek `free` / `htop`'takinden düşük görünüyor**
skry gerçekten kullanılan belleği sayar (toplam − kullanılabilir). Linux'un
gerektiğinde boşalttığı önbellekler sayılmaz.
→ [hesaplamalar](calculations.md#bellek)

**Disk beklediğimden farklı bir değer gösteriyor**
Kutucuk **en dolu** gerçek dosya sistemini gösterir. Sanal dosya sistemleri,
snap'ler, Docker katmanları ve bind mount kopyaları gizlenir.
→ [hesaplamalar](calculations.md#disk-kullanımı)

**Ağ sayıları kartın hız testinden düşük**
skry saniyede **bayt** gösterir (MiB/sn). Mbit/sn için yaklaşık 8,4 ile çarpın.

**Bekleyen güncellemeler eski görünüyor**
skry sunucunun önbellekteki paket listelerini okur ve onları asla yenilemez.
Önbelleğin en son ne zaman güncellendiğine bakın (`ls -l /var/lib/apt/lists`).

**Bir süreç %100'den fazla CPU gösteriyor**
Süreç CPU% değeri çekirdek başınadır (`top` gibi); %250 = iki buçuk çekirdek.

**Başarısız girişler iki katı görünüyor**
Var olmayan bir kullanıcıya yapılan tek deneme iki log satırı üretir. Sayı
denemeleri değil olayları sayar.

**Her şey camgöbeği (deviation)**
Baseline'lar hâlâ öğreniyor ya da filonuz gerçekten değişti.
`baseline.z_threshold`'u artırın ya da baseline'ları kapatın.
→ [Baseline'lar](baselines.md)

## Terminal arayüzü

**Kutular ve çubuklar bozuk görünüyor**
UTF-8 bir terminal ve kutu çizim karakterleri olan bir yazı tipi kullanın.
Windows'ta Windows Terminal kullanın. `LANG`'ın UTF-8 bir yerel ayar olduğunu
kontrol edin.

**Renkler yanlış / okunması zor**
skry terminalin standart renklerini kullanır; farklı bir terminal teması
seçin.

**`[` "no history recorded yet" diyor**
Geçmiş skry başladığında başlar. Ya da kapalıdır (`--no-history`,
`history.enabled = false`).

**Bir çökme sonrasında ekran karmakarışık**
Terminalde `reset` çalıştırın.

## Web paneli

**"refusing to bind to non-loopback address … without an access token"**
`SKRY_WEB_TOKEN` ya da `[web] token` belirleyin.
→ [Web](web.md#başka-makinelerden-erişim-token-kuralı)

**Tarayıcı "access token required" gösteriyor**
Adresi bir kez `?token=<token>` ile açın.

**Panel "reconnecting…" gösteriyor**
skry serve durdu ya da ağ koptu; sayfa kendiliğinden yeniden bağlanır.

## Uyarılar

**Hiç mesaj gelmiyor**
- Uyarılar yalnızca terminal arayüzü ya da `skry serve` çalışırken gönderilir.
- Webhook'taki `events`'i kontrol edin.
- Logda (`--log-file`) `webhook …: HTTP 4xx/5xx` arayın.
- Bekleme süresini (varsayılan 15 dakika) ve `unreachable_after`'ı unutmayın.

## Hâlâ takıldınız mı?

`skry --version` çıktısı, işletim sisteminiz, uzak dağıtım ve
`skry -vv --log-file …` çıktısından ilgili satırlarla (gizli bilgileri
çıkararak) bir kayıt açın: <https://github.com/enderkus/skry/issues>
