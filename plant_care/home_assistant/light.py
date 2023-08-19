from __future__ import annotations

import requests

from homeassistant.core import HomeAssistant
from homeassistant.components.light import (
    ATTR_BRIGHTNESS,
    LightEntity,
    ColorMode,
)
from homeassistant.config_entries import ConfigEntry
from homeassistant.const import CONF_HOST

async def async_setup_entry(
    hass: HomeAssistant,
    entry: ConfigEntry,
    async_add_entities
) -> None:
    data = entry.as_dict()
    host = data['data'][CONF_HOST]
    async_add_entities([Light(host)])
    return True

class Light(LightEntity):
    def __init__(self, host) -> None:
        self._url = f'http://{host}/power/0'
        self._name = f'{host} LED'
        self._brightness = None
        self._supported_color_modes = set(ColorMode.BRIGHTNESS)

    @property
    def name(self) -> str:
        return self._name

    @property
    def brightness(self):
        return self._brightness

    @property
    def color_mode(self):
        return ColorMode.BRIGHTNESS

    @property
    def supported_color_modes(self):
        return self._supported_color_modes

    @property
    def is_on(self) -> bool | None:
        return self._brightness != 0

    def turn_on(self, **kwargs: Any) -> None:
        self._brightness = int(requests.put(self._url, str(kwargs.get(ATTR_BRIGHTNESS, 255))).text)

    def turn_off(self, **kwargs: Any) -> None:
        self._brightness = int(requests.put(self._url, '0').text)

    def update(self) -> None:
        self._brightness = int(requests.get(self._url).text)
