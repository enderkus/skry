# Güvenlik modeli

Bu sayfa skry'nin neye karşı koruduğunu, nasıl koruduğunu ve sınırların
nerede olduğunu listeler.

## Sözler ve nasıl tutuldukları

**Sunucularınıza hiçbir şey kurulmaz ya da yazılmaz.**
skry normal bir SSH oturumu üzerinden ölçüm başına salt okunur tek bir POSIX
kabuk betiği gönderir. Dosya yüklemez, geçici dosya oluşturmaz, arka plan
süreci başlatmaz, yetki yükseltmez. Entegrasyon testleri bunu denetler: bir
test sunucusunda değiştirilen her dosyayı kaydeder, birkaç toplama yapar ve
listenin değişmediğini doğrular. (Sizin kontrolünüzdeki istisnalar: sshd'nin
girişinizi loglaması ve `rpm_updates`'i açarsanız dnf'in kendi logları.)

**Keyfi komut çalıştırmanın yolu yok.**
skry'de bilerek "bu komutu tüm sunucularda çalıştır" özelliği yoktur. Uzak bir
kabuğa ulaşan tek kullanıcı metni `skry find service` içindeki birim adıdır ve
bir şey gönderilmeden önce `[A-Za-z0-9@._:-]` kümesine göre doğrulanır. Geri
kalan her şey (`find port`, `find proc`) sizin bilgisayarınızda süzülür.

**Bölüm sınırları taklit edilemez.**
Betiğin bölümlerini ayıran işaretler, her ölçümde yeniden seçilen rastgele
64 bitlik bir değer içerir. Sunucudaki kötü niyetli bir süreç adı, log satırı ya
da konteyner adı sahte bir bölüm ekleyemez.

**Sunucu kimliği doğrulanır.**
Host anahtarları `known_hosts`'a göre denetlenir (hash'lenmiş girdiler, joker
karakterler, `@revoked`, özel dosyalar ve `HostKeyAlias` desteklenir).
`--accept-new` verilmedikçe bilinmeyen anahtarlar reddedilir; değişmiş
anahtarlar her zaman reddedilir. Atlama sunucuları da aynı şekilde doğrulanır.
Bkz. [Host anahtarları](host-keys.md).

**Parola yok.**
Yalnızca açık anahtarla kimlik doğrulama desteklenir. Anahtar parolaları bir
kez sorulur, anahtarı bellekte çözmek için kullanılır; asla saklanmaz ya da
loglanmaz.

**Anahtarlarınız sizde kalır.**
ProxyJump, bastion üzerinden TCP yönlendirmesi kullanır; hedef oturum sizinle
hedef arasında uçtan uca şifrelidir. Ajan yönlendirmesi asla kullanılmaz.

**Gizli bilgiler maskelenir.**
Webhook adresleri ve web token'ı loglarda ya da hata ayıklama çıktısında asla
görünmez; loglar `https://hooks.slack.com/<redacted>` gösterir.

**Web paneli varsayılan olarak kilitlidir.**
Yalnızca `127.0.0.1` üzerinde dinler. Başka yerde dinlemek bir token ister.
Token karşılaştırması sabit zamanlıdır. Yalnızca `GET` yolları vardır. Yanıtlar
sıkı bir Content-Security-Policy (`default-src 'self'`),
`X-Frame-Options: DENY`, `X-Content-Type-Options: nosniff` ve
`Referrer-Policy: no-referrer` taşır. Sunuculardan gelen tüm metinler (süreç
adları, konteyner adları, mesajlar) HTML olarak değil metin olarak eklenir;
kötü niyetli bir isim tarayıcınızda betik çalıştıramaz.

## Tehditler ve sınırlar

| Tehdit | Koruma | Sınır |
| --- | --- | --- |
| Biri bir sunucunun kılığına giriyor | host anahtarı doğrulama | İlk temasta parmak izini kontrol etmeden `--accept-new` ile bir anahtarı kabul ederseniz, o an cevap veren neyse ona güvenmiş olursunuz |
| Ele geçirilmiş bir sunucu sahte veri gönderiyor | ayrıştırıcılar katıdır ve bir şey çalıştırmaya kandırılamaz; işaretler taklit edilemez | Ele geçirilmiş bir sunucu elbette kendi metrikleri hakkında yalan söyleyebilir |
| Çalınan izleme anahtarı | ayrı, yetkisiz bir hesap kullanın; `authorized_keys`'te `restrict` | Anahtar yine giriş yapabilir ve o hesabın okuyabildiklerini okuyabilir |
| Ağdaki biri paneli okuyor | varsayılan olarak loopback, aksi hâlde token zorunlu | skry düz HTTP sunar: güvenilmeyen ağlarda bir TLS ters vekil sunucu ya da SSH tüneli kullanın |
| Bilgisayarınızdaki diğer kullanıcılar | yapılandırma ve geçmiş, ev dizininizdeki normal dosyalardır | Dosya izinleriyle koruyun (`chmod 600 config.toml`) |

## Yerelde neler saklanır

- Yapılandırma dosyası (webhook adreslerini ve web token'ını içerebilir).
- Geçmiş veritabanı: metrikler, süreç ve komut adları, konteyner adları,
  başarısız girişlerin kaynak adresleri. Diğer izleme verileri gibi davranın.
- `--accept-new` ile eklenen `known_hosts` girdileri.
- Oluşturduğunuz anlık görüntü dosyaları.

Bkz. [skry'nin bilgisayarınızda tuttuğu dosyalar](data.md).

## Güvenlik açığı bildirmek

Güvenlik sorunlarını lütfen
[GitHub güvenlik duyuruları](https://github.com/enderkus/skry/security/advisories/new)
üzerinden gizli olarak bildirin.
