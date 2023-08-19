from homeassistant.core import HomeAssistant
from homeassistant.config_entries import ConfigEntry
from homeassistant.const import CONF_HOST, Platform

import logging
import requests

from .const import DOMAIN

_LOGGER = logging.getLogger(__name__)

PLATFORMS = [Platform.LIGHT, Platform.SWITCH]

async def async_setup_entry(hass: HomeAssistant, entry: ConfigEntry):
    for p in PLATFORMS:
        hass.async_create_task(
            hass.config_entries.async_forward_entry_setup(entry, p)
        )

    return True

async def async_unload_entry(hass: HomeAssistant, entry: ConfigEntry):
    for p in PLATFORMS:
        await hass.config_entries.async_forward_entry_unload(entry, p)

    return True

class PlantCareEntity:
    def __init__(self, host: str, idx: int):
        self._host = host
        self._url = f'http://{host}/power/{idx}'
        escaped_host = self._host.replace('.', '_')
        self._base_unique_id = f'plant_care.{escaped_host}'
        self._device_info = {
            "identifiers": {
                ("id", self._base_unique_id)
            },
            "name": f"PlantCare [{self._host}]",
        }


    @property
    def device_info(self):
        return self._device_info

    def set_power(self, power: int) -> int:
        return int(requests.put(self._url, str(power)).text)

    def get_power(self) -> int:
        return int(requests.get(self._url).text)

