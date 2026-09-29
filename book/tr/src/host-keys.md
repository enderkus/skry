# Host anahtarları ve known_hosts

## Host anahtarları neden önemli?

`web1`'e bağlandığınızda, gerçekten `web1` ile konuştuğunuzu, trafiğinizi başka
yere yönlendiren bir saldırganla konuşmadığınızı nereden bilirsiniz? Sunucu
kimliğini **host anahtarıyla** kanıtlar. Bilgisayarınız bilinen host
anahtarlarının listesini `~/.ssh/known_hosts` dosyasında tutar. Anahtar
eşleşiyorsa bu gerçekten sizin sunucunuzdur.

skry, OpenSSH ile aynı kuralları sıkı biçimde uygular.

## Olası üç sonuç

| Durum | `--accept-new` olmadan | `--accept-new` ile |
| --- | --- | --- |
| Anahtar `known_hosts`'ta var ve eşleşiyor | bağlan | bağlan |
| Sunucu `known_hosts`'ta hiç **yok** | **reddet** ("unknown host key") | anahtarı kaydet, sonra bağlan |
| Sunucu `known_hosts`'ta **farklı** bir anahtarla var | **reddet** ("HOST KEY MISMATCH") | **reddet** — her zaman |
| Anahtar `@revoked` olarak işaretli | **reddet** | **reddet** |

`--accept-new` tam olarak OpenSSH'nin `StrictHostKeyChecking=accept-new`
ayarı gibi davranır: **ilk temas** için güvenlidir ve bir uyuşmazlığı asla
geçersiz kılmaz.

## Yeni sunucularla ilk temas

Seçenek A — bir kez `ssh` ile bağlanın ve parmak izini kontrol ettikten sonra
"yes" deyin. skry bundan sonra o sunucuya güvenir.

Seçenek B — anahtarları skry kaydetsin:

```sh
skry --accept-new @yeni-sunucular
```

Gerçek güvenlik için skry'nin gösterdiği parmak izini (`SHA256:...`) sunucuda
yazdırılanla karşılaştırın:

```sh
# sunucuda, ör. bulut konsolu üzerinden
ssh-keygen -lf /etc/ssh/ssh_host_ed25519_key.pub
```

Yeni anahtarlar ilk `known_hosts` dosyasının (normalde `~/.ssh/known_hosts`)
sonuna `[203.0.113.10]:2222 ssh-ed25519 AAAA…` gibi sıradan satırlar olarak
eklenir. skry'yi aynı anda çok sayıda yeni sunucuya karşı çalıştırırsanız
yazmalar sıraya konur; satırlar asla birbirine karışmaz.

## "HOST KEY MISMATCH" — şimdi ne olacak?

```text
HOST KEY MISMATCH for web1: server offered SHA256:abc…, which differs from
/home/siz/.ssh/known_hosts:17; possible man-in-the-middle, refusing to connect
```

Zararsızdan ciddiye doğru olası nedenler:

1. Sunucu **yeniden kuruldu** ya da SSH anahtarları yeniden üretildi.
2. Adres artık **başka bir makineye** ait (bulut IP'leri yeniden kullanılır).
3. Biri bağlantınızın **arasına giriyor**.

Satırı körü körüne silmeyin. Anahtarın gerçekten değiştiğini sunucunun sahibiyle
ya da konsoldan teyit edin, sonra eski girdiyi silip yeniden bağlanın:

```sh
ssh-keygen -R web1                     # port 22
ssh-keygen -R "[203.0.113.10]:2222"    # diğer portlar
skry --accept-new web1
```

Mesaj, eski anahtarın hangi dosyada ve kaçıncı satırda olduğunu söyler.

## skry'nin known_hosts'ta anladıkları

- Düz sunucu adları ve adresler: `web1,203.0.113.10 ssh-ed25519 AAAA…`
- Standart olmayan portlar: `[203.0.113.10]:2222 ssh-ed25519 AAAA…`
- `HashKnownHosts yes` ile yazılan **hash'lenmiş** girdiler (`|1|…|…`)
  (Debian ve Ubuntu'da varsayılan)
- Joker karakterler ve hariç tutma: `*.example.com,!evil.example.com …`
- `@revoked` işaretleri
- Birden çok dosya: `~/.ssh/known_hosts` ve `~/.ssh/known_hosts2` ya da
  `UserKnownHostsFile` ne gösteriyorsa
- `~/.ssh/config`'teki `HostKeyAlias`

Desteklenmeyen: `@cert-authority` (SSH host sertifikaları) satırları yok
sayılır. Sunucularınız yalnızca host sertifikası kullanıyorsa düz anahtarlarını
ekleyin ya da bir kez `--accept-new` ile bağlanın.

## Anahtar türleri

`known_hosts`'ta bir sunucu için örneğin ECDSA anahtarı zaten varsa skry,
tıpkı OpenSSH gibi, sunucudan önce ECDSA anahtarını ister; böylece birden çok
anahtar türüne sahip bir sunucu yanlış bir uyuşmazlığa yol açmaz.

## Atlama sunucuları

Bir bastion üzerinden bağlandığınızda hem bastion hem hedef doğrulanır.
Hedefin anahtarı, bastion'ın bağlandığı isimle (örneğin `db1` ya da
`10.0.5.20`) aranır; bu yüzden `known_hosts`'ta o isimle bulunmalıdır.
