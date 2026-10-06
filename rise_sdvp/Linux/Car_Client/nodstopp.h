/*
    nodstopp.h — läser nödstoppsknappen (NC-brytare) på en GPIO-pinne på Pi:n.

    Kopplingen: NC-kontakten mellan GPIO-pinnen och GND, intern pull-up i Pi:n.
      sluten kontakt (knappen ute)        -> pinnen låg  -> OK
      bruten kontakt (intryckt/kabelbrott) -> pinnen hög  -> NÖDSTOPP

    Använder kärnans GPIO-gränssnitt (v2, /dev/gpiochip*) direkt, utan bibliotek, så att
    samma kod fungerar på Pi 4 och Pi 5 (pinnen letas upp på namnet "GPIO<n>").
*/

#ifndef NODSTOPP_H
#define NODSTOPP_H

#include <QString>

class NodstoppGpio
{
public:
    NodstoppGpio() {}
    ~NodstoppGpio();

    // Öppnar GPIO<n> som ingång med pull-up. false + feltext om det inte går.
    bool open(int gpio, QString &fel);
    // 1 = kontakten sluten (OK), 0 = bruten (nödstopp), -1 = läsfel.
    int lasSluten();

private:
    int mLineFd = -1;
};

#endif // NODSTOPP_H
