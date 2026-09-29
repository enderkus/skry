# Sık sorulan sorular

## Genel

**Sunuculara gerçekten hiçbir şey kurmam gerekmiyor mu?**
Evet. skry'nin yalnızca sshd'ye ve her Linux sunucusunda zaten bulunan bir
POSIX kabuğuna ihtiyacı vardır. Hiçbir şey kopyalanmaz, kurulmaz ya da
çalışır hâlde bırakılmaz.

**skry sunucularıma bir şey yazıyor mu?**
Hayır. Betik yalnızca okur. Yazılan tek şey skry'nin kontrolü dışındadır: sshd,
her SSH girişinde olduğu gibi sizin girişinizi de loglar. `rpm_updates`'i
açarsanız dnf kendi log dosyalarına yazar — bu yüzden o seçenek varsayılan
olarak kapalıdır.

**skry root ya da sudo istiyor mu?**
Hayır. Normal bir hesap yeter. Bazı veriler bir grup üyeliği ister (loglar,
Docker). → [İzinler](permissions.md)

**skry benim yerime sunucularımda komut çalıştırabilir mi?**
Hayır, bilerek. skry salt okunurdur ve filoda keyfi komut çalıştırma özelliği
yoktur.

**Hangi sunucuları izleyebilirim?**
SSH ile erişilebilen Linux sunucularını: Debian, Ubuntu, RHEL/Rocky/Alma,
Alpine ve akrabaları. → [Uyumluluk](compatibility.md)

**Windows ya da BSD sunucularını izleyebilir miyim?**
Hayır. skry Linux'un `/proc`'unu okur.

**skry nerede çalışır?**
Sizin bilgisayarınızda: Linux, macOS ya da Windows.

**Prometheus + node_exporter, Zabbix, Netdata… gibi araçlardan farkı ne?**
Onlar her sunucuya bir ajan kurar ve verileri merkezde uzun süre saklar. skry
hiçbir şey kurmaz, saniyeler içinde başlar ve canlı resme artı 24 saatlik
geçmişe odaklanır. Birbirlerini tamamlarlar: skry Prometheus için `/metrics`
bile sunabilir.

**skry ücretsiz mi?**
Evet; tercihinize göre MIT ya da Apache-2.0 lisanslı açık kaynak.

## Bağlanma

**Her seferinde anahtar parolamı soruyor.**
Anahtarı SSH ajanınıza yükleyin (`ssh-add`); skry bir daha sormaz.

**Anahtar yerine parola kullanabilir miyim?**
Hayır. Parolalı girişler güvenli biçimde otomatikleştirilemez ve skry asla
parola saklamaz. Bir anahtar kurun (`ssh-keygen`, `ssh-copy-id`).
→ [SSH temelleri](ssh-basics.md)

**skry `~/.ssh/config` dosyamı kullanıyor mu?**
Evet: `Host`, `HostName`, `User`, `Port`, `IdentityFile`, `IdentitiesOnly`,
`ProxyJump`, `Include`, `UserKnownHostsFile`, `HostKeyAlias`,
`ConnectTimeout`. Diğer seçenekler yok sayılır.

**`ProxyCommand` destekleniyor mu?**
Hayır, `ProxyJump` kullanın. → [Bastion](proxyjump.md)

**`--accept-new` güvenli mi?**
`ssh`'in ilk sorduğunda "yes" demek kadar güvenlidir. *Değişmiş* bir anahtarı
asla kabul etmez. Tam güvenlik için parmak izini sunucudakiyle karşılaştırın.

**skry kaç SSH girişi üretir?**
Her skry oturumunda sunucu başına bir tane (artı hatalardan sonraki yeniden
bağlanmalar) — ölçüm başına bir tane değil.

**skry bir SOCKS ya da HTTP vekil sunucusu üzerinden çalışır mı?**
Doğrudan değil. `ProxyJump` ile bir bastion ya da bir VPN kullanın.

## Veriler ve sayılar

**CPU ve ağ neden bir saniye `n/a` gösteriyor?**
Bunlar hızdır: skry'nin iki ölçüme ihtiyacı vardır. → [Her ölçüm](tick.md)

**Bellek kullanımım neden başka araçlardakinden düşük?**
skry boşaltılabilir önbelleği kullanılmış saymaz.
→ [hesaplamalar](calculations.md#bellek)

**CPU % çekirdek başına mı, bütün makine için mi?**
Sunucu CPU %'si tüm çekirdekleri kapsar (%100 = hepsi meşgul). Süreç CPU %'si
çekirdek başınadır (`top` gibi).

**Dinleyen portlarda UDP neden yok?**
skry yalnızca TCP dinleyicilerini kontrol eder.

**Bekleyen güncellemeler listesi ne kadar taze?**
Sunucunun kendi paket önbelleği kadar; skry asla `apt update` çalıştırmaz.

**Çalıştırdığım bir konteyner neden garip CPU/bellek sayıları gösteriyor?**
Bir konteyneri izlerseniz sayılar çoğunlukla ana makinenin çekirdeğini anlatır.
Ana makineyi izleyin.
→ [Uyumluluk](compatibility.md#hedef-olarak-konteynerler-ve-sanal-makineler)

**Veriler ne sıklıkla toplanıyor?**
Varsayılan olarak her 2 sn'de bir; portlar, konteynerler, birimler, girişler
ve güncellemeler 60 sn'de bir. → [Her ölçüm](tick.md)

## Geçmiş, baseline'lar, uyarılar

**Geçmiş nerede saklanıyor ve ne kadar büyük?**
Bilgisayarınızdaki bir SQLite dosyasında; 24 saat için sunucu başına yaklaşık
1–3 MB. → [Geçmiş](history.md)

**Geçmişi 24 saatten uzun tutabilir miyim?**
Evet: `history.retention_hours`. Uzun süreli saklama için `/metrics`'i
Prometheus ile toplayın.

**skry çalışmıyorken uyarı gönderir mi?**
Hayır. Kesintisiz uyarı için `skry serve`'ü bir servis olarak çalıştırın.
→ [Otomasyon](automation.md)

**Neden "çözüldü" mesajı almadım?**
Ya `notify_resolved = false`, ya da asıl uyarı bekleme süresi yüzünden
bastırıldı (o zaman çözülecek bir şey yoktur).

**"Deviation" (sapma) nedir?**
skry'nin öğrendiklerine göre o sunucu için olağandışı olan bir değer.
→ [Baseline'lar](baselines.md)

## Web ve Prometheus

**Çalışma arkadaşlarım paneli açabilir mi?**
Evet; bir ağ adresine bağlayın ve bir token belirleyin. TLS arkasına koyun.
→ [Web](web.md)

**Panelden ayarları değiştirebilir miyim?**
Hayır. Salt okunurdur.

**`/metrics` için panel gerekli mi?**
İkisi de `skry serve`'ün parçasıdır; birlikte gelirler.

## Gizlilik

**skry verileri bir yere gönderiyor mu?**
Yalnızca sizin söylediğiniz yere: webhook'larınıza. Telemetri yoktur. TLS
kontrolü yapılandırdığınız uç noktalara bağlanır; kurulum betiği GitHub'a
bağlanır.
