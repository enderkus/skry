# Bastion ve ProxyJump

Birçok ağ, SSH'ye yalnızca sağlamlaştırılmış tek bir giriş sunucusu
(**bastion** ya da atlama sunucusu) üzerinden izin verir. skry bunu standart
`ProxyJump` ayarıyla, tam olarak `ssh -J` gibi destekler.

## Kurulum

`~/.ssh/config` içinde:

```text
Host bastion
    HostName bastion.example.com
    User ops

Host db1 db2 10.0.5.*
    ProxyJump bastion
    User postgres
```

Artık `skry db1 db2` önce bastion'a bağlanır ve `db1` ile `db2`'ye onun
üzerinden ulaşır. Bastion'da da ek bir şey gerekmez: yalnızca TCP
yönlendirmesine izin vermesi yeterlidir (`AllowTcpForwarding yes`, OpenSSH
varsayılanı; Alpine'in varsayılan yapılandırmasının bunu `no` yaptığını
unutmayın).

## Nasıl çalışır

```text
bilgisayarınız ══SSH══▶ bastion ──SSH içinde TCP tüneli──▶ db1:22
      └──────────────── uçtan uca ikinci SSH oturumu ─────────┘
```

1. skry bastion'a bir SSH bağlantısı açar, host anahtarını doğrular ve giriş
   yapar.
2. Bu bağlantı üzerinden bastion'dan `db1:22`'ye bir TCP tüneli açmasını ister
   ("direct-tcpip" kanalı — `ssh -J`'nin yaptığıyla aynı şey).
3. Tünelin içinden doğrudan `db1` ile **ikinci, eksiksiz bir SSH oturumu**
   yürütür: kendi host anahtarı kontrolü, kendi girişi. Bastion yalnızca
   şifreli baytları taşır; okuyamaz.
4. Anahtarlarınız bilgisayarınızdan asla çıkmaz. Ajan yönlendirmesi
   kullanılmaz.

Hedef bağlantısı açık olduğu sürece bastion bağlantısı da açık kalır.

## Zincirler

`ProxyJump` virgülle ayrılmış birden çok atlama kabul eder ve bir atlama
sunucusunun kendisinin de `ProxyJump`'ı olabilir:

```text
Host ic
    ProxyJump kenar,orta
```

skry zinciri sırayla izler (kenar → orta → ic), en fazla 8 atlamaya kadar.
Bir döngü (A, B üzerinden; B, A üzerinden atlıyor) algılanır ve yapılandırma
hatası olarak bildirilir.

`ProxyJump none`, aksi hâlde bir joker kurala uyacak bir sunucu için atlamayı
kapatır.

## Bilinmesi gerekenler

- **Her atlamanın kendi zaman aşımı vardır** (`connect_timeout`, varsayılan
  10 sn); atlama sayısı arttıkça izin verilen toplam süre de artar.
- **Hatalar nerede olduğunu söyler**: `via jump host bastion: authentication failed …`,
  hedefin değil bastion'ın sizi reddettiği anlamına gelir.
- **Bastion arkasındaki hedeflerin host anahtarları**, bastion'ın bağlandığı
  isimle (hedefin `HostName`'i ya da ismin kendisi) aranır. Bir kez
  `--accept-new` kullanın ya da bir kez `ssh` ile bağlanın.
- **`ProxyCommand` desteklenmez.** Yaygın durumları kapsayan `ProxyJump`'ı
  kullanın (`ssh -W %h:%p bastion`, `ProxyJump bastion` ile eşdeğerdir).
