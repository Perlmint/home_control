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

from . import PlantCareEntity

async def async_setup_entry(
    hass: HomeAssistant,
    entry: ConfigEntry,
    async_add_entities
) -> None:
    data = entry.as_dict()
    host = data['data'][CONF_HOST]
    async_add_entities([Light(host)])
    return True

class Light(PlantCareEntity, LightEntity):
    def __init__(self, host) -> None:
        super().__init__(host, 0)
        self._brightness = None

    @property
    def unique_id(self):
        return f'{self._base_unique_id}.light'

    @property
    def name(self) -> str:
        return f'{self._host} LED'

    @property
    def brightness(self):
        return self._brightness

    @property
    def color_mode(self):
        return ColorMode.BRIGHTNESS

    @property
    def supported_color_modes(self):
        return set(ColorMode.BRIGHTNESS)

    @property
    def is_on(self) -> bool | None:
        return self._brightness != 0

    def turn_on(self, **kwargs: Any) -> None:
        self._brightness = self.set_power(kwargs.get(ATTR_BRIGHTNESS, 255))

    def turn_off(self, **kwargs: Any) -> None:
        self._brightness = self.set_power(0)

    def update(self) -> None:
        self._brightness = self.get_power()

