from __future__ import annotations

from homeassistant.components.switch import (
    SwitchEntity,
)
from homeassistant.const import CONF_HOST

import logging

from . import PlantCareEntity

_LOGGER = logging.getLogger(__name__)

async def async_setup_entry(
    hass: HomeAssistant,
    entry: ConfigEntry,
    async_add_entities
) -> None:
    data = entry.as_dict()
    host = data['data'][CONF_HOST]
    entity = WaterPump(host)
    _LOGGER.info(f'water pump created {host}')
    async_add_entities([entity])
    return True

class WaterPump(PlantCareEntity, SwitchEntity):
    def __init__(self, host):
        super().__init__(host, 1)
        self._value = None

    @property
    def unique_id(self):
        return f'{self._base_unique_id}.water_pump'

    @property
    def name(self):
        return f'{self._host} water pump'

    @property
    def is_on(self):
        return self._value != 0

    def turn_on(self):
        self._value = self.set_power(255)

    def turn_off(self):
        self._value = self.set_power(0)

    def update(self):
        self._value = self.get_power()

    @property
    def icon(self):
        return "mdi:water-pump"

